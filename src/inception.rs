//! Put references to objects in other objects... in an object.
//!
//! This builds high level encodings on top of `chaos`, things like large object
//! storage, compression, delta compression, and encryption.  All these high
//! level operations are very deliberately kept out of `chaos`.

use std::slice::Iter;
use std::io::prelude::*;
use std::collections::HashSet;
use std::{io, fs, cmp};
use zstd::stream::{Encoder, Decoder};
use crate::base::*;
use crate::protocol::Hasher;
use crate::chaos::{Object, Store, Name};
use std::marker::PhantomData;


#[derive(Debug)]
pub struct LeafHashes<const N: usize> {
    total: u64,
    hashes: Vec<Name<N>>,
}

impl<const N: usize> LeafHashes<N> {
    pub fn new() -> Self {
        Self {total: 0, hashes: Vec::new()}
    }

    pub fn iter(&self) -> Iter<Name<N>> {
        self.hashes.iter()
    }

    pub fn append_leaf(&mut self, hash: Name<N>, size: usize) {
        self.hashes.push(hash);
        self.total += size as u64;
    }

    pub fn serialize(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.total.to_le_bytes());
        for hash in self.hashes.iter() {
            buf.extend_from_slice(hash.as_buf());
        }
    }

    pub fn deserialize(buf: &[u8]) -> Self {
        let total = u64::from_le_bytes(buf[0..8].try_into().expect("oops"));
        let mut hashes: Vec<Name<N>> = Vec::new();
        let mut offset = 8;
        while offset < buf.len() {
            let hash = Name::from(&buf[offset..offset + N]);
            hashes.push(hash);
            offset += N;
        }
        assert_eq!(offset, buf.len());
        Self {total: total, hashes: hashes}
    }
}


pub fn hash_file<H: Hasher, const N: usize> (
        obj: &mut Object<H, N>,
        mut file: fs::File,
        size: u64
    ) -> io::Result<Name<N>> {
    if size == 0 {
        panic!("No good, yo, your size is ZERO!");
    }
    if size > OBJECT_MAX_SIZE as u64 {
        let mut leaves = LeafHashes::<N>::new();
        let mut remaining = size;
        while remaining > 0 {
            let s = cmp::min(remaining, OBJECT_MAX_SIZE as u64);
            remaining -= s;
            obj.reset(s as usize, 0);
            file.read_exact(obj.as_mut_data())?;
            leaves.append_leaf(obj.finalize(), obj.info().size());
        }
        obj.clear();
        leaves.serialize(obj.as_mut_vec());
        Ok(obj.finalize_with_kind(1))
    }
    else {
        obj.reset(size as usize, 0);
        file.read_exact(obj.as_mut_data())?;
        Ok(obj.finalize())
    }
}


pub fn import_file<H: Hasher, const N: usize>(
        store: &mut Store<H, N>,
        obj: &mut Object<H, N>,
        mut file: fs::File,
        size: u64
    ) -> io::Result<Name<N>> {
    if size == 0 {
        panic!("No good, yo, your size is ZERO!");
    }
    if size > OBJECT_MAX_SIZE as u64 {
        let mut leaves = LeafHashes::<N>::new();
        let mut remaining = size;
        while remaining > 0 {
            let s = cmp::min(remaining, OBJECT_MAX_SIZE as u64);
            remaining -= s;
            obj.reset(s as usize, 0);
            file.read_exact(obj.as_mut_data())?;
            leaves.append_leaf(obj.finalize(), obj.info().size());
            store.save(&obj)?;
        }
        obj.clear();
        leaves.serialize(obj.as_mut_vec());
        let root = obj.finalize_with_kind(1);
        store.save(&obj)?;
        Ok(root)
    }
    else {
        obj.reset(size as usize, 0);
        file.read_exact(obj.as_mut_data())?;
        let hash = obj.finalize();
        store.save(&obj)?;
        Ok(hash)
    }
}

pub fn restore_file<H: Hasher, const N: usize> (
        store: &mut Store<H, N>,
        obj: &mut Object<H, N>,
        file: &mut fs::File,
        root: &Name<N>,
    ) -> io::Result<bool> {
    if store.load(root, obj)? {
        let kind = obj.info().kind();
        match kind {
            0 => {
                file.write_all(obj.as_data())?;
            }
            1 => {
                let hashes = LeafHashes::<N>::deserialize(obj.as_data());
                for hash in hashes.iter() {
                    if store.load(&hash, obj)? {
                        file.write_all(obj.as_data())?;
                    }
                    else {
                        panic!("Cannot find {} leaf {}", root, hash);
                    }
                }
            }
            _ => {
                panic!("No good, yo, no good at all! 😵‍💫");
            }
        }
        Ok(true)
    }
    else {
        Ok(false)
    }
}


/// Compress an object stream and store in inside of an object.
/// 
/// 16 MiB is a lot of compressed source code, so typically all objects in a
/// DVCS commit will fit within a single object.  We can beat git on initial
/// space efficiency this way: objects will compress much better back to back
/// in the same compression stream.  It also means we can write a commit with a
/// single call to `Store.save()`.
pub struct Compress<H: Hasher, const N: usize> {
    total: usize,
    set: HashSet<Name<N>>,
    encoder: Encoder<'static, Object<H, N>>,
}

impl<H: Hasher, const N: usize> Compress<H, N>  {
    pub fn new(mut obj: Object<H, N>) -> io::Result<Self> {
        obj.clear();
        Ok( Self {
            total: 0,
            set: HashSet::new(),
            encoder: Encoder::new(obj, 0)?,
        })
    }

    pub fn has_space(&self, obj: &Object<H, N>) -> bool {
        // FIXME: what we really want is the free space in the underlying object,
        // but the currest Rust zstd API doen't seem to offer this
        OBJECT_MAX_SIZE - self.total > obj.len()
    }

    pub fn push(&mut self, obj: &Object<H, N>) -> io::Result<bool> {
        assert!(self.has_space(obj));
        let hash = obj.hash();
        if self.set.contains(&hash) {
            Ok(false)
        }
        else {
            self.set.insert(hash);
            self.total += obj.len();
            self.encoder.write_all(obj.as_buf())?;
            Ok(true)
        }
    }

    pub fn finish(self) -> io::Result<Object<H, N>> {
        self.encoder.finish()
    }
}


pub struct Decompress<R: io::BufRead, H: Hasher, const N: usize> {
    phantom: PhantomData<H>,
    decoder: Decoder<'static, R>,
}

impl<R: io::BufRead, H: Hasher, const N: usize> Decompress<R, H, N>
{
/*
    pub fn new(mut inner: R) -> io::Result<Self> {
        let decoder: Decoder<'static, R> = Decoder::new(inner)?;
        Ok( Self {
            phantom: PhantomData,
            decoder: decoder,
        })
    }
*/
    pub fn read_next(&mut self, obj: &mut Object<H, N>) -> io::Result<bool> {
        Ok(true)
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Blake3;

    #[test]
    fn test_compress() {
        let inner: Object<Blake3, 30> = Object::new();
        let mut comp = Compress::new(inner).unwrap();
        let mut obj: Object<Blake3, 30> = Object::new();
        while comp.has_space(&obj) {
            obj.as_mut_vec().extend_from_slice(
                b"It's all about control systems, baby."
            );
            obj.finalize();
            comp.push(&obj);
        }
        let inner = comp.finish().unwrap();
        assert!(inner.as_data().len() < OBJECT_MAX_SIZE);
        assert_eq!(inner.as_data().len(), 38071);
    }
}

