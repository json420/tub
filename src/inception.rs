//! Put references to objects in other objects in an object.  Builds on `chaos`.
//!
//! This builds high level encodings on top of `chaos`, things like large object
//! storage, compression, delta compression, and encryption.  All these high
//! level operations are very deliberately kept out of `chaos`.

use std::slice::Iter;
use std::io::prelude::*;
use std::{io, fs, cmp};
use crate::base::*;
use crate::protocol::Hasher;
use crate::chaos::{Object, Store, Name};


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



pub struct FileStream<H: Hasher, const N: usize> {
    store: Store<H, N>,
}

impl<H: Hasher, const N: usize> FileStream<H, N> {
    pub fn new(store: Store<H, N>) -> Self {
        Self {store: store}
    }

    pub fn import_file(&mut self, obj: &mut Object<H, N>, mut file: fs::File, size: u64) -> io::Result<Name<N>> {
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
                self.store.save(&obj)?;
            }
            obj.clear();
            leaves.serialize(obj.as_mut_vec());
            let root = obj.finalize();
            self.store.save(&obj)?;
            Ok(root)
        }
        else {
            obj.reset(size as usize, 0);
            file.read_exact(obj.as_mut_data())?;
            let hash = obj.finalize();
            self.store.save(&obj)?;
            Ok(hash)
        }
    }

    pub fn restore_file(
        &mut self,
        root: &Name<N>,
        obj: &mut Object<H, N>,
        file: &mut fs::File) -> io::Result<bool>
    {
        if self.store.load(root, obj)? {
            let kind = obj.info().kind();
            match kind {
                0 => {
                    file.write_all(obj.as_data())?;
                }
                1 => {
                    let hashes = LeafHashes::<N>::deserialize(obj.as_data());
                    for hash in hashes.iter() {
                        if self.store.load(&hash, obj)? {
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
}


#[cfg(test)]
mod tests {
    #[test]
    fn test_stuff() {
    
    }
}
