//! Content Hash Addressable Object Store (START HERE).
//!
//! An Object's wire format contains just for fields of the following sizes in
//! bytes:
//!
//! | Hash | Size | Kind | DATA       |
//! |------|------|------|------------|
//! |   30 |    3 |    1 | 1-16777216 |
//!
//! All objects start with a fixed size header (generic on `<N>`) followed by
//! 1 to 16777216 bytes of object data.  An object with a size of zero is not
//! allow and is not even expressible in the header.
//!
//! The size is "size minus one" encoded into 24 bits by first subtracting one
//! from the size.  In 24 bits you can store values from 0-16777215, but what
//! we actually want is 1-16777216, both so the zero value is not possible
//! to represent, and so the high value is that nice power of 2.
//!
//!


use crate::base::*;
use crate::protocol::{Hasher, Blake3};
use crate::dbase32::db32enc;
use crate::util::getrandom;
use std::{fs, io, path, cmp};
use std::collections::HashMap;
use std::os::unix::fs::FileExt;
use std::fmt;
use std::io::prelude::*;
use std::slice::Iter;



/// N byte long Tub name (content hash or random ID).
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Name<const N: usize> {
    pub buf: [u8; N],
}

impl<const N: usize> Name<N> {
    pub fn new() -> Self {
        Self {buf: [0_u8; N]}
    }

    pub fn from(src: &[u8]) -> Self {
        let buf: [u8; N] = src.try_into().expect("oops");
        Self {buf: buf}
    }

    pub fn into_buf(self) -> [u8; N] {
        self.buf
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn to_string(&self) -> String {
        db32enc(&self.buf)
    }
}

impl<const N: usize> fmt::Display for Name<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

pub type TubId2 = Name<15>;


/// Packs 24-bit `size` and 8-bit `kind` into a `u32`.
#[derive(Debug, PartialEq, Eq)]
pub struct Info {
    val: u32,
}

impl Info {
    fn new(size: usize, kind: u8) -> Self {
        if size < 1 || size > OBJECT_MAX_SIZE {
            panic!("Info: Need 1 <= size <= {}; got size={}", OBJECT_MAX_SIZE, size);
        }
        Self {val: (size - 1) as u32 | (kind as u32) << 24}
    }

    pub fn from_le_bytes(buf: &[u8]) -> Self {
        Self {val: u32::from_le_bytes(buf.try_into().expect("oops"))}
    }

    pub fn to_le_bytes(&self) -> [u8; 4] {
        self.val.to_le_bytes()
    }

    pub fn raw(&self) -> u32 {
        self.val
    }

    pub fn size(&self) -> usize {
        ((self.val & 0x00ffffff) + 1) as usize
    }

    pub fn kind(&self) -> u8 {
        (self.val >> 24) as u8
    }
}


/// Buffer containing a single object's header plus data.
#[derive(Debug)]
pub struct Object<H: Hasher, const N: usize> {
    hasher: H,
    buf: Vec<u8>,
}

impl<H: Hasher, const N: usize> Object<H, N> {
    pub fn new() -> Self {
        Self {
            buf: vec![0; N + INFO_LEN],
            hasher: H::new(),
        }
    }

    pub fn reset(&mut self, size: usize, kind: u8) {
        self.buf.clear();
        self.buf.resize(N + INFO_LEN + size, 0);
        self.set_info(Info::new(size, kind));
    }
    
    // FIXME: remove this soon
    pub fn resize(&mut self, size: usize) {
        self.buf.clear();
        self.buf.resize(N + INFO_LEN + size, 0);
    }

    pub fn clear(&mut self) {
        self.buf.clear();
        self.buf.resize(N + INFO_LEN, 0);
    }

    pub fn resize_to_info(&mut self) {
        self.buf.resize(N + INFO_LEN + self.info().size(), 0);
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    // FIXME: We should not have this in the API, but super handy for testing and play
    pub fn randomize(&mut self, small: bool) -> Name<N> {
        getrandom(&mut self.buf[N..N + INFO_LEN]);
        if small {
            self.buf[N + 1] = 0;
            self.buf[N + 2] = 0;
        }
        self.resize_to_info();
        getrandom(self.as_mut_data());
        self.finalize()
    }

    pub fn extend_from_slice(&mut self, newstuff: &[u8]) {
        self.buf.extend_from_slice(newstuff);
    }

    pub fn compute(&self) -> Name<N> {
        let mut hash: Name<N> = Name::new();
        self.hasher.hash_into(
            self.info().raw(), self.as_data(),
            hash.as_mut_buf()
        );
        hash
    }

    pub fn is_valid(&self) -> bool {
        self.hash() == self.compute()
    }

    pub fn validate_against(&self, hash: &Name<N>) -> bool {
        self.is_valid() && hash == &self.hash()
    }

    pub fn finalize(&mut self) -> Name<N> {
        self.set_info(Info::new(self.as_data().len(), 69));
        assert_eq!(self.buf.len(), N + INFO_LEN + self.info().size());
        let hash = self.compute();
        self.buf[0..N].copy_from_slice(hash.as_buf());
        hash
    }

    pub fn hash(&self) -> Name<N> {
        Name::from(&self.buf[0..N])
    }

    pub fn set_hash(&mut self, hash: Name<N>) {
        self.buf[0..N].copy_from_slice(hash.as_buf());
    }

    pub fn info(&self) -> Info {
        Info::from_le_bytes(&self.buf[N..N + INFO_LEN])
    }

    pub fn set_info(&mut self, info: Info) {
        self.buf[N..N + INFO_LEN].copy_from_slice(&info.to_le_bytes());
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.buf
    }   

    pub fn as_mut_header(&mut self) -> &mut [u8] {
        &mut self.buf[0..N + INFO_LEN]
    }

    pub fn as_data(&self) -> &[u8] {
        &self.buf[N + INFO_LEN..]
    }

    pub fn as_mut_data(&mut self) -> &mut [u8] {
        &mut self.buf[N + INFO_LEN..]
    }

    pub fn hash_file(&mut self, mut file: fs::File, size: u64) -> io::Result<Name<N>> {
        if size == 0 {
            panic!("No good, yo, your size is ZERO!");
        }
        if size > OBJECT_MAX_SIZE as u64 {
            let mut tree: Object<H, N> = Object::new();
            let mut remaining = size;
            while remaining > 0 {
                let s = cmp::min(remaining, OBJECT_MAX_SIZE as u64);
                remaining -= s;
                self.reset(s as usize, 0);
                file.read_exact(self.as_mut_data())?;
                tree.extend_from_slice(self.finalize().as_buf());
            }
            Ok(tree.finalize())
        }
        else {
            self.reset(size as usize, 0);
            file.read_exact(self.as_mut_data())?;
            Ok(self.finalize())
        }
    }
}

pub type DefaultObject = Object<Blake3, 30>;

/// A value in the `Store.map` HashMap index.
pub struct Entry {
    pub info: Info,
    pub offset: u64,
}

impl Entry {
    pub fn new(info: Info, offset: u64) -> Self {
        Self {info: info, offset: offset}
    }
}


/// Organizes objects in an append-only file.
pub struct Store<H: Hasher, const N: usize> {
    file: fs::File,
    _hasher: H,
    map: HashMap<Name<N>, Entry>,
    offset: u64,
}

impl<H: Hasher, const N: usize> Store<H, N> {
    pub fn new(file: fs::File) -> Self {
        Self {
            file: file,
            _hasher: H::new(),
            map: HashMap::new(),
            offset: 0,
        }
    }

    pub fn new_object(&self) -> Object<H, N> {
        Object::new()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn keys(&self) -> Vec<Name<N>> {
        Vec::from_iter(self.map.keys().cloned())
    }

    pub fn reindex(&mut self, obj: &mut Object<H, N>) -> io::Result<()> {
        self.map.clear();
        self.offset = 0;
        obj.resize(0);
        while let Ok(_) = self.file.read_exact_at(obj.as_mut_header(), self.offset) {
            obj.resize_to_info();
            let offset = self.offset + (N + INFO_LEN) as u64;
            if let Ok(_) = self.file.read_exact_at(obj.as_mut_data(), offset) {
                if obj.is_valid() {
                    let hash = obj.hash();
                    let entry = Entry::new(obj.info(), self.offset);
                    self.map.insert(hash, entry);
                }
                else {
                    panic!("shitballs {}", offset);
                }
                self.offset += (N + INFO_LEN + obj.info().size()) as u64;
            }
            obj.resize(0);
        }
        Ok(())
    }

    pub fn load(&mut self, hash: &Name<N>, obj: &mut Object<H, N>) -> io::Result<bool> {
        if let Some(entry) = self.map.get(hash) {
            obj.resize(entry.info.size());
            self.file.read_exact_at(obj.as_mut_buf(), entry.offset)?;
            if ! obj.validate_against(hash) {
                panic!("{} hash does not match", hash);
            }
            Ok(true)
        }
        else {
            Ok(false)
        }
    }

    pub fn save(&mut self, obj: &Object<H, N>) -> io::Result<bool> {
        assert!(obj.is_valid());
        let hash = obj.hash();
        let info = obj.info();
        if let Some(_entry) = self.map.get(&hash) {
            Ok(false)
        }
        else {
            self.file.write_all(obj.as_buf())?;
            self.map.insert(hash, Entry::new(info, self.offset));
            self.offset += obj.len() as u64;
            Ok(true)
        }
    }

    pub fn delete(&mut self, _hash: Name<N>) -> io::Result<bool> {
        // FIXME: Decide how tombstones should work with new new
        Ok(true)
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
                //tree.extend_from_slice(obj.finalize().as_buf());
                leaves.append_leaf(obj.finalize(), obj.info().size());
                self.save(&obj)?;
            }
            obj.clear();
            leaves.serialize(obj.as_mut_vec());
            let root = obj.finalize();
            self.save(&obj)?;
            Ok(root)
        }
        else {
            obj.reset(size as usize, 0);
            file.read_exact(obj.as_mut_data())?;
            let hash = obj.finalize();
            self.save(&obj)?;
            Ok(hash)
        }
    }

    pub fn restore_file(&mut self, root: &Name<N>, obj: &mut Object<H, N>, file: &mut fs::File) -> io::Result<bool>
    {
        if self.load(root, obj)? {
            let kind = obj.info().kind();
            match kind {
                0 => {
                    file.write_all(obj.as_data())?;
                }
                1 => {
                    let hashes = LeafHashes::<N>::deserialize(obj.as_data());
                    for hash in hashes.iter() {
                        if self.load(&hash, obj)? {
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
        assert_eq!(offset, buf.len() + 8);
        Self {total: total, hashes: hashes}
    }
}


pub type DefaultStore = Store<Blake3, 30>;



#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::{TestTempDir, flip_bit_in};

    #[test]
    fn test_name() {
        let mut n = Name::<30>::new();
        assert_eq!(n.len(), 30);
        assert_eq!(n.as_buf(), [0_u8; 30]);
        assert_eq!(n.as_mut_buf(), [0_u8; 30]);
        assert_eq!(n.to_string(), "333333333333333333333333333333333333333333333333");
        n.as_mut_buf().fill(255);
        assert_eq!(n.len(), 30);
        assert_eq!(n.as_buf(), [255_u8; 30]);
        assert_eq!(n.as_mut_buf(), [255_u8; 30]);
        assert_eq!(n.to_string(), "YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY");

        let mut n = Name::<20>::new();
        assert_eq!(n.len(), 20);
        assert_eq!(n.as_buf(), [0_u8; 20]);
        assert_eq!(n.as_mut_buf(), [0_u8; 20]);
        assert_eq!(n.to_string(), "33333333333333333333333333333333");
        n.as_mut_buf().fill(255);
        assert_eq!(n.len(), 20);
        assert_eq!(n.as_buf(), [255_u8; 20]);
        assert_eq!(n.as_mut_buf(), [255_u8; 20]);
        assert_eq!(n.to_string(), "YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY");
    }

    #[test]
    fn test_info() {
        let info = Info::from_le_bytes(&[0; 4]);
        assert_eq!(info.size(), 1);
        assert_eq!(info.kind(), 0);
        assert_eq!(info.raw(), 0);
        assert_eq!(info.to_le_bytes(), [0; 4]);

        let info = Info::from_le_bytes(&[255; 4]);
        assert_eq!(info.size(), OBJECT_MAX_SIZE);
        assert_eq!(info.kind(), 255);
        assert_eq!(info.raw(), u32::MAX);
        assert_eq!(info.to_le_bytes(), [255; 4]);

        let info = Info::new(1, 0);
        assert_eq!(info.size(), 1);
        assert_eq!(info.kind(), 0);
        assert_eq!(info.raw(), 0);
        assert_eq!(info.to_le_bytes(), [0; 4]);

        let info = Info::new(OBJECT_MAX_SIZE, 255);
        assert_eq!(info.size(), OBJECT_MAX_SIZE);
        assert_eq!(info.kind(), 255);
        assert_eq!(info.raw(), u32::MAX);
        assert_eq!(info.to_le_bytes(), [255; 4]);
    }

    #[test]
    #[should_panic(expected="Need 1 <= size <= 16777216; got size=0")]
    fn test_info_panic_low() {
        let _sk = Info::new(0, 0);
    }

    #[test]
    #[should_panic(expected="Need 1 <= size <= 16777216; got size=16777217")]
    fn test_info_panic_high() {
        let _sk = Info::new(OBJECT_MAX_SIZE + 1, 0);
    }

    #[test]
    fn test_object() {
        let mut obj: Object<Blake3, 30> = Object::new();
        assert_eq!(obj.len(), 34);
        obj.resize(0);
        assert_eq!(obj.len(), 34);

        assert_eq!(obj.info().size(), 1);
        assert_eq!(obj.info().kind(), 0);
        assert_eq!(obj.as_buf(), &[0; 34]);

        obj.as_mut_buf().fill(255);
        assert_eq!(obj.info().size(), 16 * 1024 * 1024);
        assert_eq!(obj.info().kind(), 255);

        assert_eq!(obj.len(), 34);
        assert_eq!(obj.as_buf(), &[255; 34]);

        obj.resize(0);
        assert_eq!(obj.len(), 34);
        assert_eq!(obj.as_buf(), &[0; 34]);
    }

    #[test]
    fn test_object_validity() {
        let mut obj: Object<Blake3, 30> = Object::new();
        obj.randomize(true);
        assert!(obj.is_valid());
        for bit in 0..obj.len() * 8 {
            flip_bit_in(obj.as_mut_buf(), bit);
            assert!(! obj.is_valid());
            flip_bit_in(obj.as_mut_buf(), bit);
            assert!(obj.is_valid());
        }

        let mut hash = obj.hash();
        for bit in 0..hash.len() * 8 {
            flip_bit_in(hash.as_mut_buf(), bit);
            assert!(! obj.validate_against(&hash));
            flip_bit_in(hash.as_mut_buf(), bit);
            assert!(obj.validate_against(&hash));
        }
    }

    #[test]
    fn test_store() {
        let tmp = TestTempDir::new();
        let path = tmp.build(&["foo"]);
        let file = fs::File::options().read(true).append(true).create(true).open(&path).unwrap();
        let mut store = Store::<Blake3, 30>::new(file);
        let mut obj = store.new_object();
        store.reindex(&mut obj).unwrap();

        let mut obj1 = store.new_object();
        let mut obj2 = store.new_object();

        for _ in 0..8 {
            obj1.randomize(false);
            let hash1 = obj1.hash();
            assert!(store.save(&obj1).unwrap());
            assert!(store.map.contains_key(&hash1));
            obj2.resize(0);
            assert!(store.load(&hash1, &mut obj2).unwrap());
            assert_eq!(obj1.as_buf(), obj2.as_buf());
        }
        for _ in 0..256 {
            obj1.randomize(true);
            let hash1 = obj1.hash();
            assert!(store.save(&obj1).unwrap());
            assert!(store.map.contains_key(&hash1));
            obj2.resize(0);
            assert!(store.load(&hash1, &mut obj2).unwrap());
            assert_eq!(obj1.as_buf(), obj2.as_buf());
        }

        let keys = store.keys();
        for key in keys.iter() {
            assert!(store.load(&key, &mut obj1).unwrap());
        }
        store.reindex(&mut obj1).unwrap();
    }
}

