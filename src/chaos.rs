//! Content Hash Addressable Object Store (START HERE).
//!
//! Dead simple.

use crate::base::*;
use crate::protocol::{Hasher, Blake3};
use crate::dbase32::db32enc;
use crate::util::getrandom;
use std::{fs, io, path};
use std::collections::HashMap;
use std::os::unix::fs::FileExt;
use std::fmt;
use std::io::prelude::*;
/*

Current protocol V0 object format:

| HASH | INFO | PAYLOAD             |
|   30 |    4 | 1 - MAX_OBJECT_SIZE |

*/


// FIXME: Can we put compile time contraints on N such that N > 0 && N % 5 == 0?
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TubName<const N: usize> {
    pub buf: [u8; N],
}

impl<const N: usize> TubName<N> {
    pub fn new() -> Self {
        Self {buf: [0_u8; N]}
    }

    pub fn from(src: &[u8]) -> Self {
        let buf: [u8; N] = src.try_into().expect("oops");
        Self {buf: buf}
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

impl<const N: usize> fmt::Display for TubName<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

pub type TubId2 = TubName<15>;


/// Packs 24-bit `size` and 8-bit `kind` into a u32
#[derive(Debug, PartialEq, Eq)]
pub struct Info {
    val: u32,
}

impl Info {
    fn new(size: usize, kind: u8) -> Self {
        if size < 1 || size > OBJECT_MAX_SIZE {
            panic!("Need 1 <= size <= {}; got size={}", OBJECT_MAX_SIZE, size);
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

    pub fn resize(&mut self, size: usize) {
        self.buf.clear();
        self.buf.resize(N + INFO_LEN + size, 0);
    }

    pub fn resize_to_info(&mut self) {
        self.buf.resize(N + INFO_LEN + self.info().size(), 0);
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    // FIXME: We should not have this in the API, but super handy for testing and play
    pub fn randomize(&mut self, small: bool) -> TubName<N> {
        getrandom(&mut self.buf[N..N + INFO_LEN]);
        if small {
            self.buf[N + 1] = 0;
            self.buf[N + 2] = 0;
        }
        self.resize_to_info();
        getrandom(self.as_mut_data());
        self.finalize()
    }

    pub fn compute(&self) -> TubName<N> {
        let mut hash: TubName<N> = TubName::new();
        self.hasher.hash_into(
            self.info().raw(), self.as_data(),
            hash.as_mut_buf()
        );
        hash
    }

    pub fn is_valid(&self) -> bool {
        self.hash() == self.compute()
    }

    pub fn validate_against(&self, hash: &TubName<N>) -> bool {
        self.is_valid() && hash == &self.hash()
    }

    pub fn finalize(&mut self) -> TubName<N> {
        assert_eq!(self.buf.len(), N + INFO_LEN + self.info().size());
        let hash = self.compute();
        self.buf[0..N].copy_from_slice(hash.as_buf());
        hash
    }

    pub fn hash(&self) -> TubName<N> {
        TubName::from(&self.buf[0..N])
    }

    pub fn set_hash(&mut self, hash: TubName<N>) {
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

    pub fn as_mut_vec(&mut self) -> &Vec<u8> {
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
}

pub struct Entry {
    info: Info,
    offset: u64,
}

impl Entry {
    pub fn new(info: Info, offset: u64) -> Self {
        Self {info: info, offset: offset}
    }
}


pub fn open_for_store(path: &path::Path) -> io::Result<fs::File> {
    fs::File::options().read(true).append(true).create(true).open(path)
}


/// Organizes objects in an append-only file
pub struct Store<H: Hasher, const N: usize> {
    file: fs::File,
    hasher: H,
    map: HashMap<TubName<N>, Entry>,
    offset: u64,
}

impl<H: Hasher, const N: usize> Store<H, N> {
    pub fn new(file: fs::File) -> Self {
        Self {
            file: file,
            hasher: H::new(),
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

    pub fn reindex(&mut self, obj: &mut Object<H, N>) -> io::Result<()> {
        self.map.clear();
        self.offset = 0;
        obj.resize(0);
        while let Ok(_) = self.file.read_exact_at(obj.as_mut_header(), self.offset) {
            println!("{} {} {} {}",
                obj.hash(),
                self.offset,
                obj.info().size(),
                obj.info().kind(),
            );
            obj.resize_to_info();
            let offset = self.offset + (N + INFO_LEN) as u64;
            if let Ok(_) = self.file.read_exact_at(obj.as_mut_data(), offset) {
                if obj.is_valid() {
                    let hash = obj.hash();
                    let entry = Entry::new(obj.info(), self.offset);
                    self.map.insert(hash, entry);
                }
                self.offset += (N + INFO_LEN + obj.info().size()) as u64;
            }
            obj.resize(0);
        }
        Ok(())
    }

    pub fn load(&mut self, hash: &TubName<N>, obj: &mut Object<H, N>) -> io::Result<bool> {
        if let Some(entry) = self.map.get(hash) {
            obj.resize(entry.info.size());
            if let Ok(_) = self.file.read_exact_at(obj.as_mut_buf(), entry.offset) {
                if obj.validate_against(hash) {
                    Ok(true)
                }
                else {
                    obj.resize(0);
                    Ok(false)
                }
            }
            else {
                obj.resize(0);
                Ok(false)
            }
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
            self.offset += info.size() as u64;
            self.map.insert(hash, Entry::new(info, self.offset));
            Ok(true)
        }
    }

    pub fn delete(&mut self, _hash: TubName<N>) -> io::Result<bool> {
        // FIXME: Decide how tombstones should work with new new
        Ok(true)
    }
}


pub type StoreHblake3N30 = Store<Blake3, 30>;


#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::{TestTempDir, flip_bit_in};

    #[test]
    fn test_tubname() {
        let mut n = TubName::<30>::new();
        assert_eq!(n.len(), 30);
        assert_eq!(n.as_buf(), [0_u8; 30]);
        assert_eq!(n.as_mut_buf(), [0_u8; 30]);
        assert_eq!(n.to_string(), "333333333333333333333333333333333333333333333333");
        n.as_mut_buf().fill(255);
        assert_eq!(n.len(), 30);
        assert_eq!(n.as_buf(), [255_u8; 30]);
        assert_eq!(n.as_mut_buf(), [255_u8; 30]);
        assert_eq!(n.to_string(), "YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY");

        let mut n = TubName::<20>::new();
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
        let file = fs::File::create(&path).unwrap();
        let mut store = Store::<Blake3, 30>::new(file);
        let mut obj = store.new_object();
        store.reindex(&mut obj).unwrap();
    }
}

