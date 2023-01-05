//! Test fixtures.

// (FIXME: should eventually be put somewhere else).

use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::fs;
use tempfile;




pub struct TestTempDir {
    tmp: tempfile::TempDir,
}


impl TestTempDir {
    pub fn new() -> Self {
        Self {
            tmp: tempfile::TempDir::new().unwrap(),
        }
    }

    pub fn path(&self) -> &Path {
        self.tmp.path()
    }

    pub fn pathbuf(&self) -> PathBuf {
        self.tmp.path().to_path_buf()
    }

    // Construct an absolute path starting with self.path()
    pub fn build(&self, names: &[&str]) -> PathBuf {
        let mut pb = self.pathbuf();
        for n in names {
            pb.push(n);
        }
        pb
    }

    pub fn list_dir(&self, parts: &[&str]) -> Vec<String> {
        let path = self.build(parts);
        let mut names: Vec<String> = Vec::new();
        if let Ok(entries) = fs::read_dir(path) {
            for result in entries {
                if let Ok(entry) = result {
                    names.push(
                        entry.file_name().into_string().unwrap()
                    );
                    println!("{:?}", entry.file_name());
                }
            }
        }
        names.sort();
        names
    }

    pub fn list_root(&self) -> Vec<String> {
        self.list_dir(&[])
    }

    pub fn read(&self, names: &[&str]) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        fs::File::open(self.build(names)).unwrap().read_to_end(&mut buf).unwrap();
        buf
    }

    pub fn touch(&self, names: &[&str]) {
        fs::File::create(self.build(names)).unwrap();
    }

    pub fn write(&self, names: &[&str], data: &[u8]) {
        fs::File::create(self.build(names)).unwrap().write_all(data).unwrap();
    }

    pub fn append(&self, names: &[&str], data: &[u8]) {
        fs::File::options().append(true).open(self.build(names)).unwrap()
            .write_all(data).unwrap();
    }

    pub fn mkdir(&self, names: &[&str]) -> PathBuf {
        let pb = self.build(names);
        fs::create_dir(&pb).unwrap();
        pb
    }

    pub fn makedirs(&self, names: &[&str]) -> PathBuf {
        let pb = self.build(names);
        fs::create_dir_all(&pb).unwrap();
        pb
    }
}


pub fn flip_bit_in(buf: &mut [u8], bit: usize) {
    assert!(bit < buf.len() * 8);
    buf[bit / 8] ^= 1<<(bit % 8);
}

pub fn flip_bit(src: &[u8], bit: usize) -> Vec<u8> {
    assert!(bit < src.len() * 8);
    let mut copy = Vec::from(src);
    flip_bit_in(&mut copy, bit);
    copy
}


#[derive(Debug)]
pub struct BitFlipIter<'a> {
    src: &'a [u8],
    bit: usize,
}

impl<'a> BitFlipIter<'a> {
    pub fn new(src: &'a [u8]) -> Self {
        Self {src: src, bit: 0}
    }
}

impl<'a> Iterator for BitFlipIter<'a> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bit < self.src.len() * 8 {
            let buf = flip_bit(self.src, self.bit);
            self.bit += 1;
            Some(buf)
        }
        else {
            None
        }
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use super::*;
    use crate::util::getrandom;

    #[test]
    fn test_tempdir() {
        let tmp = TestTempDir::new();
        let base = tmp.path();
        let a = tmp.build(&["a"]);
        assert!(a.to_str().unwrap().starts_with(base.to_str().unwrap()));
        assert!(a.to_str().unwrap().ends_with("/a"));

        assert_eq!(tmp.list_root().len(), 0);
        tmp.touch(&["a"]);
        assert_eq!(tmp.list_root(), vec!["a"]);
    
    }

    #[test]
    fn test_flip_bit() {
        let src = [0_u8; 2];
        assert_eq!(flip_bit(&src,  0), vec![0b00000001, 0b00000000]);
        assert_eq!(flip_bit(&src,  1), vec![0b00000010, 0b00000000]);
        assert_eq!(flip_bit(&src,  2), vec![0b00000100, 0b00000000]);
        assert_eq!(flip_bit(&src,  3), vec![0b00001000, 0b00000000]);
        assert_eq!(flip_bit(&src,  4), vec![0b00010000, 0b00000000]);
        assert_eq!(flip_bit(&src,  5), vec![0b00100000, 0b00000000]);
        assert_eq!(flip_bit(&src,  6), vec![0b01000000, 0b00000000]);
        assert_eq!(flip_bit(&src,  7), vec![0b10000000, 0b00000000]);
        assert_eq!(flip_bit(&src,  8), vec![0b00000000, 0b00000001]);
        assert_eq!(flip_bit(&src,  9), vec![0b00000000, 0b00000010]);
        assert_eq!(flip_bit(&src, 10), vec![0b00000000, 0b00000100]);
        assert_eq!(flip_bit(&src, 11), vec![0b00000000, 0b00001000]);
        assert_eq!(flip_bit(&src, 12), vec![0b00000000, 0b00010000]);
        assert_eq!(flip_bit(&src, 13), vec![0b00000000, 0b00100000]);
        assert_eq!(flip_bit(&src, 14), vec![0b00000000, 0b01000000]);
        assert_eq!(flip_bit(&src, 15), vec![0b00000000, 0b10000000]);

        let src = [255_u8; 2];
        assert_eq!(flip_bit(&src,  0), vec![0b11111110, 0b11111111]);
        assert_eq!(flip_bit(&src,  1), vec![0b11111101, 0b11111111]);
        assert_eq!(flip_bit(&src,  2), vec![0b11111011, 0b11111111]);
        assert_eq!(flip_bit(&src,  3), vec![0b11110111, 0b11111111]);
        assert_eq!(flip_bit(&src,  4), vec![0b11101111, 0b11111111]);
        assert_eq!(flip_bit(&src,  5), vec![0b11011111, 0b11111111]);
        assert_eq!(flip_bit(&src,  6), vec![0b10111111, 0b11111111]);
        assert_eq!(flip_bit(&src,  7), vec![0b01111111, 0b11111111]);
        assert_eq!(flip_bit(&src,  8), vec![0b11111111, 0b11111110]);
        assert_eq!(flip_bit(&src,  9), vec![0b11111111, 0b11111101]);
        assert_eq!(flip_bit(&src, 10), vec![0b11111111, 0b11111011]);
        assert_eq!(flip_bit(&src, 11), vec![0b11111111, 0b11110111]);
        assert_eq!(flip_bit(&src, 12), vec![0b11111111, 0b11101111]);
        assert_eq!(flip_bit(&src, 13), vec![0b11111111, 0b11011111]);
        assert_eq!(flip_bit(&src, 14), vec![0b11111111, 0b10111111]);
        assert_eq!(flip_bit(&src, 15), vec![0b11111111, 0b01111111]);
    }

    #[test]
    fn test_bit_flip_iter() {
        let mut set: HashSet<Vec<u8>> = HashSet::new();
        let src = vec![0; 2];
        for dif in BitFlipIter::new(&src) {
            let new = set.insert(dif);
            assert!(new);
        }
        assert_eq!(set.len(), 16);
        set.insert(src);
        assert_eq!(set.len(), 17);

        let mut set: HashSet<Vec<u8>> = HashSet::new();
        let mut src = vec![0; 69];
        getrandom(&mut src);
        for dif in BitFlipIter::new(&src) {
            let new = set.insert(dif);
            assert!(new);
        }
        assert_eq!(set.len(), 69 * 8);
        set.insert(src);
        assert_eq!(set.len(), 69 * 8 + 1);
    }
}

