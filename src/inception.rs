//! Put references to objects in other objects... in an object.
//!
//! This builds high level encodings on top of `chaos`, things like large object
//! storage, compression, delta compression, and encryption.  All these high
//! level operations are very deliberately kept out of `chaos`.

use std::slice::Iter;
use std::io::prelude::*;
use std::collections::HashSet;
use std::{io, fs, cmp};
//use zstd::stream::{Encoder, Decoder};
use crate::base::*;
use crate::protocol::Hasher;
use crate::chaos::{Object, Store, Name, ObjectReader};
use std::marker::PhantomData;


/*
We want a generalized Container type that stores an encoded object stream,
where the encoding is any combination of delta compression, general compression,
and encryption (and chained in that order for encoding, reverse that order
for decoding).

    Encode: --> Delta   --> Compress   --> Encrypt -->
    Decode: <-- Dedelta <-- Decompress <-- Decrypt <--

We'll use three bytes to specify the encoding:

    | Delta Byte | Compress Byte | Encrypt Byte |

A value of 0 in a field means do nothing (pass through).  A Delta byte of 1
means general delta, 2 means document delta, and so on.

We'll have at least two types of delta compression: "general" (basically what
Git does) and "document" (a special high performance content aware delta format
used to specify changes between document revisions).

Delta compression should always be combined with general compression.
Obviously zstd will be the default cuz that algorithm kicks fuckin' ass.  But to
make sure the protocols are truly future friendly and allow us add additional
general compression formats, let's have at least two from the get go to make us
fully work through the problem.  The other algorithm should offer a better
compression ratio than zstd (so it's going to be slower), but lets still pick
the best performance we can get for the compression ratio.

We should also offer a couple of encryption algorithms out of the gate, for the
same reason.  Let's keep the protocol design iterations well constraining
within practical engineering realities.  Design through implementation.  If
the implementation keeps turning into shit, then the design is shit and we
should iterate on the design again.

Containers will not be allowed in other containers (the nesting is at most
one level deep).

Everything is an object.  Put objects back to back (no other framing needed)
and then you have yourself on objects stream.  Super duper Goddamn elegant and
simple, yo.

Next we need some kind of tree object to lookup which object contains the
requested object.  If an object is not directly stored in the store (which we
can test quickly with Store.load()), then we need lookup in this tree.  We'll be
aggressively iterating on these details for a while, so hold on, partner!
*/



/*
We should probably define a trait for a common object stream interface, one we
use whether the steam is being decoded out of a container object, read out of
a file, or recv'd over a socket.  And have the same for the write direction.
*/

pub trait Stream<R: Read, H: Hasher, const N: usize> {
    fn new(inner: R) -> Self;
    fn send(&mut self, obj: &Object<H, N>) -> io::Result<()>;
    fn recv(&mut self, obj: &mut Object<H, N>) -> io::Result<()>;
}


#[derive(Debug)]
pub struct Container<H: Hasher, const N: usize> {
    inner: Object<H, N>,
}

impl<H: Hasher, const N: usize> Container<H, N> {
    pub fn new(inner: Object<H, N>) -> Self {
        Self {inner: inner}
    }

    pub fn has_space(&self, obj: &Object<H, N>) -> bool {
        obj.len() < self.inner.remaining()
    }
}


// Wrapper around Object, implements Read trait to read from Object data.
#[derive(Debug)]
pub struct ReadFromObject<H: Hasher, const N: usize> {
    obj: Object<H, N>,
    pos: usize,
}

impl<H: Hasher, const N: usize> ReadFromObject<H, N> {
    pub fn new(obj: Object<H, N>) -> Self {
        Self {obj: obj, pos: 0}
    }

    pub fn into_inner(self) -> Object<H, N> {
        self.obj
    }
}

impl<H: Hasher, const N: usize> io::Read for ReadFromObject<H, N> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data = self.obj.as_data();
        let remaining = data.len() - self.pos;
        let amount = cmp::min(remaining, buf.len());
        if amount > 0 {
            let start = self.pos;
            let stop = start + amount;
            self.pos = stop;
            assert!(self.pos <= data.len());
            buf[0..amount].copy_from_slice(&data[start..stop]);
            Ok(amount)
        }
        else {
            Ok(0)
        }
    }
}


// Wrapper around Object, implements Write trait to write into Object data.
#[derive(Debug)]
pub struct WriteToObject<H: Hasher, const N: usize> {
    obj: Object<H, N>,
}

impl<H: Hasher, const N: usize> io::Write for WriteToObject<H, N>
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let remaining = cmp::min(buf.len(), self.obj.remaining());
        if remaining > 0 {
            self.obj.as_mut_vec().extend_from_slice(&buf[0..remaining]);
            Ok(remaining)
        }
        else {
            Ok(0)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}



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
/// space efficiency this way when there are mulitple new objects in a commit
/// (objects will compress much better back to back in the same compression
/// stream).  It also means we can write a commit with a single call to
/// `Store.save()`.




#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::getrandom;
    use crate::protocol::Blake3;

    #[test]
    fn test_rfo_empty() {
        let obj: Object<Blake3, 30> = Object::new();
        let mut rfo = ReadFromObject::new(obj);
        let mut buf = [0; 69];
        assert_eq!(rfo.read(&mut buf).unwrap(), 0);
        assert_eq!(buf, [0; 69]);
    }

    #[test]
    fn test_rfo_42() {
        let mut buf = [0; 69];
        let mut obj: Object<Blake3, 30> = Object::new();
        let mut data = [0; 42];
        getrandom(&mut data);
        obj.as_mut_vec().extend_from_slice(&data);
        let mut rfo = ReadFromObject::new(obj);
        assert_eq!(rfo.read(&mut buf).unwrap(), 42);
        assert_eq!(buf[0..42], data);
        assert_eq!(buf[42..69], [0; 27]);
        buf.fill(0);
        assert_eq!(rfo.read(&mut buf).unwrap(), 0);
        assert_eq!(buf, [0; 69]);
    }

    #[test]
    fn test_rfo_max() {
        let mut obj: Object<Blake3, 30> = Object::new();
        obj.reset(OBJECT_MAX_SIZE, 0);
        getrandom(obj.as_mut_data());
        let mut rfo = ReadFromObject::new(obj);
        let mut buf = [0; 69];
        let mut output = Vec::new();
        while let Ok(s) = rfo.read(&mut buf) {
            if s == 0 {
                break;
            }
            output.extend_from_slice(&buf[0..s]);
        }
        buf.fill(0);
        assert_eq!(rfo.read(&mut buf).unwrap(), 0);
        assert_eq!(buf, [0; 69]);
        let obj = rfo.into_inner();
        assert_eq!(&output, obj.as_data());
    }
}

