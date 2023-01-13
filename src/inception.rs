//! Put references to objects in other objects... in an object.
//!
//! This builds high level encodings on top of `chaos`, things like large object
//! storage, compression, delta compression, and encryption.  All these high
//! level operations are very deliberately kept out of `chaos`.

use std::slice::Iter;
use std::io::prelude::*;
use std::collections::{HashMap, HashSet};
use std::{io, fs, cmp};
use zstd;
use crate::base::*;
use crate::protocol::Hasher;
use crate::chaos::{Object, Store, Name};
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
pub struct ReadFrom<H: Hasher, const N: usize> {
    obj: Object<H, N>,
    pos: usize,
}

impl<H: Hasher, const N: usize> ReadFrom<H, N> {
    pub fn new(obj: Object<H, N>) -> Self {
        Self {obj: obj, pos: 0}
    }

    pub fn into_inner(self) -> Object<H, N> {
        self.obj
    }
}

impl<H: Hasher, const N: usize> io::Read for ReadFrom<H, N> {
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
pub struct WriteTo<H: Hasher, const N: usize> {
    obj: Object<H, N>,
}

impl<H: Hasher, const N: usize> WriteTo<H, N> {
    pub fn new(obj: Object<H, N>) -> Self {
        Self {obj: obj}
    }

    pub fn into_inner(self) -> Object<H, N> {
        self.obj
    }
}

impl<H: Hasher, const N: usize> io::Write for WriteTo<H, N>
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



// FIXME: This is currently way the fuck too slow (but is ok for now).
pub struct LocationMap<const N: usize> {
    map: HashMap<Name<N>, Name<N>>,
}

impl<const N: usize> LocationMap<N> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    
    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn insert(&mut self, key: Name<N>, val: Name<N>) -> Option<Name<N>> {
        self.map.insert(key, val)
    }

    pub fn get(&self, key: &Name<N>) -> Option<&Name<N>> {
        self.map.get(key)
    }

    pub fn deserialize(&mut self, buf: &[u8]) {
        assert!(buf.len() > 0);
        assert!(buf.len() % (N + N) == 0);
        self.map.clear();
        let mut offset = 0;
        while offset < buf.len() {
            let key = Name::from(&buf[offset..offset + N]);
            offset += N;
            let val = Name::from(&buf[offset..offset + N]);
            offset += N;
            self.map.insert(key, val);
        }
        assert_eq!(offset, buf.len());
    }

    pub fn serialize(&self, buf: &mut Vec<u8>) {
        let mut items = Vec::from_iter(self.map.iter());
        items.sort_by(|a, b| b.0.cmp(a.0));
        for (key, val) in items.iter() {
            buf.extend_from_slice(key.as_buf());
            buf.extend_from_slice(val.as_buf());
        }
    }
}


pub struct Fanout<H: Hasher, const N: usize> {
    table: [Option<Name<N>>; 256],
    store: Store<H, N>,
    obj: Object<H, N>,
    map: LocationMap<N>,
}

impl<H: Hasher, const N: usize> Fanout<H, N> {
    pub fn new(store: Store<H, N>, obj: Object<H, N>) -> Self {
        Self {
            table: [None; 256],
            store: store,
            obj: obj,
            map: LocationMap::new(),
        }
    }

    pub fn into_inners(self) -> (Store<H, N>, Object<H, N>) {
        (self.store, self.obj)
    }

    pub fn insert(&mut self, key: Name<N>, val: Name<N>) -> io::Result<()> {
        let i = key.as_buf()[0] as usize;
        if let Some(old) = self.table[i] {
            if self.store.load(&old, &mut self.obj)? {
                self.map.deserialize(self.obj.as_data());
                self.map.insert(key, val);
                self.obj.clear();
                self.map.serialize(self.obj.as_mut_vec());
                self.obj.finalize();
                self.store.save(&mut self.obj)?;
                self.table[i] = Some(self.obj.hash());
            }
            else {
                panic!("Crap 💩, cannot find {}", old);
            }
        }
        else {
            self.map.insert(key, val);
            self.obj.clear();
            self.map.serialize(self.obj.as_mut_vec());
            self.obj.finalize();
            self.store.save(&mut self.obj)?;
            self.table[i] = Some(self.obj.hash());
        }
        Ok(())
    }

    pub fn get(&mut self, key: &Name<N>) -> io::Result<Option<Name<N>>> {
        let i = key.as_buf()[0] as usize;
        if let Some(container) = self.table[i] {
            if self.store.load(&container, &mut self.obj)? {
                self.map.deserialize(self.obj.as_data());
                if let Some(val) = self.map.get(key) {
                    return Ok(Some(val.clone()))
                }
            }
        }
        Ok(None)
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
pub struct Encoder<H: Hasher, const N: usize> {
    phantom: PhantomData<H>,
    inner: zstd::Encoder<'static, WriteTo<H, N>>,
}

impl<H: Hasher, const N: usize> Encoder<H, N> {
    fn new(dst: Object<H, N>, level: i32) -> io::Result<Self> {
        Ok( Self {
            phantom: PhantomData,
            inner: zstd::Encoder::new(WriteTo::new(dst), level)?,
        })
    }

    fn write_next(&mut self, obj: &Object<H, N>) -> io::Result<bool> {
        self.inner.write_all(obj.as_buf())?;
        Ok(true)  // FIXME
    }

    fn finish(self) -> io::Result<Object<H, N>> {
        let mut obj = self.inner.finish()?.into_inner();
        obj.finalize();  // FIXME: How to handle kind?
        Ok(obj)
    }
}


pub struct Decoder<H: Hasher, const N: usize> {
    phantom: PhantomData<H>,
    inner: zstd::Decoder<'static, io::BufReader<ReadFrom<H, N>>>,
}

impl<H: Hasher, const N: usize> Decoder<H, N> {
    pub fn new(src: Object<H, N>) -> io::Result<Self> {
        Ok( Self {
            phantom: PhantomData,
            inner: zstd::Decoder::new(ReadFrom::new(src))?,
        })
    }

    pub fn read_next(&mut self, obj: &mut Object<H, N>) -> io::Result<bool> {
        obj.clear();
        if let Ok(_) = self.inner.read_exact(obj.as_mut_header()) {
            obj.resize_to_info();
            self.inner.read_exact(obj.as_mut_data())?;
            if ! obj.is_valid() {
                panic!("Not valid {}", obj.hash());  // FIXME: handle more better
            }
            Ok(true)
        }
        else {
            Ok(false)
        }
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::getrandom;
    use crate::protocol::Blake3;
    use crate::chaos::{DefaultName, DefaultObject, DefaultStore};
    use crate::helpers::TestTempDir;

    #[test]
    fn test_fanout() {
        let tmp = TestTempDir::new();
        let file = tmp.create(&["some_file.store"]);
        let store = DefaultStore::new(file);
        let obj = DefaultObject::new();
        let mut fanout = Fanout::new(store, obj);
        let mut hash = DefaultName::new();
        let mut cont = DefaultName::new();
        for _ in 0..1024 {
            getrandom(hash.as_mut_buf());
            assert!(fanout.get(&hash).unwrap().is_none());
            getrandom(cont.as_mut_buf());
            fanout.insert(hash.clone(), cont.clone()).unwrap();
            assert_eq!(fanout.get(&hash).unwrap().unwrap(), cont);
        }
        let (store, _obj) = fanout.into_inners();
        assert_eq!(store.len(), 1024);
    }

    #[test]
    fn test_rfo_empty() {
        let obj = DefaultObject::new();
        let mut rfo = ReadFrom::new(obj);
        let mut buf = [0; 69];
        assert_eq!(rfo.read(&mut buf).unwrap(), 0);
        assert_eq!(buf, [0; 69]);
    }

    #[test]
    fn test_rfo_42() {
        let mut buf = [0; 69];
        let mut obj = DefaultObject::new();
        let mut data = [0; 42];
        getrandom(&mut data);
        obj.as_mut_vec().extend_from_slice(&data);
        let mut rfo = ReadFrom::new(obj);
        assert_eq!(rfo.read(&mut buf).unwrap(), 42);
        assert_eq!(buf[0..42], data);
        assert_eq!(buf[42..69], [0; 27]);
        buf.fill(0);
        assert_eq!(rfo.read(&mut buf).unwrap(), 0);
        assert_eq!(buf, [0; 69]);
    }

    #[test]
    fn test_rfo_max() {
        let mut obj = DefaultObject::new();
        obj.reset(OBJECT_MAX_SIZE, 0);
        getrandom(obj.as_mut_data());
        let mut rfo = ReadFrom::new(obj);
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

    #[test]
    fn test_wto_till_full() {
        let obj = DefaultObject::new();
        let mut wto = WriteTo::new(obj);
        let mut buf = [0; 69];
        getrandom(&mut buf);
        let mut expected = Vec::new();
        while let Ok(s) = wto.write(&buf) {
            if s == 0 {
                break;
            }
            expected.extend_from_slice(&buf[0..s]);
            getrandom(&mut buf);
        }
        assert_eq!(expected.len(), OBJECT_MAX_SIZE);
        let obj = wto.into_inner();
        assert_eq!(obj.as_data(), &expected[..]);
    }

    #[test]
    fn test_zstd_roundtrip() {
        let dst = DefaultObject::new();
        let wto = WriteTo::new(dst);
        let mut enc: zstd::Encoder<'static, WriteTo<Blake3, 30>>
            = zstd::Encoder::new(wto, 0).unwrap();
        let mut obj = DefaultObject::new();
        let mut expected = Vec::new();
        for _ in 0..100 {
            obj.randomize(true);
            assert_eq!(enc.write(obj.as_buf()).unwrap(), obj.len());
            expected.extend_from_slice(obj.as_buf());
        }

        let src: DefaultObject = enc.finish().unwrap().into_inner();
        let rfo = ReadFrom::new(src);
        let mut dec: zstd::Decoder<'static, io::BufReader<ReadFrom<Blake3, 30>>>
            = zstd::Decoder::new(rfo).unwrap();
        let mut buf = vec![0; expected.len()];
        assert_eq!(dec.read(&mut buf).unwrap(), expected.len());
        assert_eq!(expected, buf);
    }

    #[test]
    fn test_container_roundtrip() {
        let inner = DefaultObject::new();
        let mut enc = Encoder::new(inner, 0).unwrap();
        let mut obj = DefaultObject::new();
        let mut expected: Vec<Vec<u8>> = Vec::new();
        for _ in 0..100 {
            obj.randomize(true);
            expected.push(Vec::from(obj.as_buf()));
            enc.write_next(&obj).unwrap();
        }
        let inner: DefaultObject = enc.finish().unwrap();
        assert!(inner.is_valid());

        let mut dec = Decoder::new(inner).unwrap();
        for i in 0..100 {
            dec.read_next(&mut obj).unwrap();
            assert!(obj.is_valid());
            assert_eq!(obj.as_buf(), &expected[i]);
        }
        assert!(! dec.read_next(&mut obj).unwrap());
        assert_eq!(obj.as_buf(), &[0; 34]);
    }
}

