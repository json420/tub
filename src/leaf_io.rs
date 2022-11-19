//! Leaf-wise File IO.
//!
//! In general, anything that uses LEAF_SIZE should be here.

use std::io;
use std::io::prelude::*;
use std::os::unix::fs::FileExt;
use std::fs::File;
use std::cmp;

use crate::base::LEAF_SIZE;
use crate::protocol;


pub fn new_leaf_buf() -> Vec<u8> {
    let mut buf = Vec::with_capacity(LEAF_SIZE as usize);
    buf.resize(LEAF_SIZE as usize, 0);
    buf
}


#[derive(Debug, PartialEq)]
pub struct LeafInfo {
    pub index: u64,
    pub size: u64,
    pub offset: u64,
}

impl LeafInfo {
    pub fn new(index: u64, size: u64, offset: u64) -> Self
    {
        Self {index: index, size: size, offset: offset}
    }
}

#[derive(Debug)]
pub struct LeafInfoIter {
    pub size: u64,
    pub offset: u64,
    index: u64,
}

impl LeafInfoIter {
    pub fn new(size: u64, offset: u64) -> Self
    {
        Self {size: size, offset: offset, index: 0}
    }
}

impl Iterator for LeafInfoIter {
    type Item = LeafInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let consumed = self.index * LEAF_SIZE;
        if consumed < self.size {
            let offset = self.offset + consumed;
            let remaining = self.size - consumed;
            let size = cmp::min(remaining, LEAF_SIZE);
            let info = LeafInfo::new(self.index, size, offset);
            self.index += 1;
            Some(info)
        }
        else {
            None
        }
    }
}


pub struct LeafReader2 {
    file: File,
    iterator: LeafInfoIter,
}

impl LeafReader2 {
    pub fn new(file: File, size: u64, offset: u64) -> Self
    {
        Self {file: file, iterator: LeafInfoIter::new(size, offset)}
    }

    pub fn read_next_leaf(&mut self, buf: &mut Vec<u8>) -> io::Result<bool>
    {
        if let Some(info) = self.iterator.next() {
            buf.resize(info.size as usize, 0);
            self.file.read_exact_at(buf, info.offset)?;
            Ok(true)
        }
        else {
            Ok(false)
        }
    }
}


pub struct LeafReader {
    file: File,
    index: u64,
    
}

impl LeafReader {
    pub fn new(file: File) -> Self
    {
        Self {file: file, index: 0}
    }

    pub fn read_next_leaf(&mut self, buf: &mut Vec<u8>) -> io::Result<Option<protocol::LeafInfo>>
    {
        buf.resize(LEAF_SIZE as usize, 0);
        let amount = self.file.read(buf)?;
        assert!(amount as u64 <= LEAF_SIZE);
        buf.resize(amount, 0);
        if amount < 1 {
            Ok(None)
        }
        else {
            let info = protocol::hash_leaf(self.index, buf);
            self.index += 1;
            Ok(Some(info))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_leaf_buf() {
        let mut buf = new_leaf_buf();
        assert_eq!(buf.len(), LEAF_SIZE as usize);
        assert_eq!(buf.capacity(), LEAF_SIZE as usize);
        //let s = &mut buf[0..111];
    }

    #[test]
    fn test_leaf_info_iter() {
        // 0
        assert_eq!(Vec::from_iter(LeafInfoIter::new(0, 0)), vec![]);
        // 1
        assert_eq!(Vec::from_iter(LeafInfoIter::new(1, 0)), vec![
            LeafInfo{index:0, size:1, offset:0},
        ]);
        // LEAF_SIZE - 1
        assert_eq!(Vec::from_iter(LeafInfoIter::new(LEAF_SIZE - 1, 0)),  vec![
            LeafInfo{index:0, size:LEAF_SIZE - 1, offset:0},
        ]);
        // LEAF_SIZE
        assert_eq!(Vec::from_iter(LeafInfoIter::new(LEAF_SIZE, 0)),  vec![
            LeafInfo{index:0, size:LEAF_SIZE, offset:0},
        ]);
        // LEAF_SIZE + 1
        assert_eq!(Vec::from_iter(LeafInfoIter::new(LEAF_SIZE + 1, 0)),  vec![
            LeafInfo{index:0, size:LEAF_SIZE, offset:0},
            LeafInfo{index:1, size:1, offset:LEAF_SIZE},
        ]);
        // 2 * LEAF_SIZE - 1
        assert_eq!(Vec::from_iter(LeafInfoIter::new(2 * LEAF_SIZE - 1, 0)),  vec![
            LeafInfo{index:0, size:LEAF_SIZE, offset:0},
            LeafInfo{index:1, size:LEAF_SIZE - 1, offset:LEAF_SIZE},
        ]);
        // 2 * LEAF_SIZE
        assert_eq!(Vec::from_iter(LeafInfoIter::new(2 * LEAF_SIZE, 0)),  vec![
            LeafInfo{index:0, size:LEAF_SIZE, offset:0},
            LeafInfo{index:1, size:LEAF_SIZE, offset:LEAF_SIZE},
        ]);
        // 2 * LEAF_SIZE + 1
        assert_eq!(Vec::from_iter(LeafInfoIter::new(2 * LEAF_SIZE + 1, 0)),  vec![
            LeafInfo{index:0, size:LEAF_SIZE, offset:0},
            LeafInfo{index:1, size:LEAF_SIZE, offset:LEAF_SIZE},
            LeafInfo{index:2, size:1, offset:2 * LEAF_SIZE},
        ]);
        // 3 * LEAF_SIZE - 1
        assert_eq!(Vec::from_iter(LeafInfoIter::new(3 * LEAF_SIZE - 1, 0)),  vec![
            LeafInfo{index:0, size:LEAF_SIZE, offset:0},
            LeafInfo{index:1, size:LEAF_SIZE, offset:LEAF_SIZE},
            LeafInfo{index:2, size:LEAF_SIZE - 1, offset:2 * LEAF_SIZE},
        ]);
        // 3 * LEAF_SIZE
        assert_eq!(Vec::from_iter(LeafInfoIter::new(3 * LEAF_SIZE, 0)),  vec![
            LeafInfo{index:0, size:LEAF_SIZE, offset:0},
            LeafInfo{index:1, size:LEAF_SIZE, offset:LEAF_SIZE},
            LeafInfo{index:2, size:LEAF_SIZE, offset:2 * LEAF_SIZE},
        ]);

        for ofst in [0_u64, 1, 2, LEAF_SIZE - 1, LEAF_SIZE, LEAF_SIZE + 1] {
            // 0
            assert_eq!(Vec::from_iter(LeafInfoIter::new(0, ofst)), vec![]);
            // 1
            assert_eq!(Vec::from_iter(LeafInfoIter::new(1, ofst)), vec![
                LeafInfo{index:0, size:1, offset:ofst},
            ]);
            // LEAF_SIZE - 1
            assert_eq!(Vec::from_iter(LeafInfoIter::new(LEAF_SIZE - 1, ofst)),  vec![
                LeafInfo{index:0, size:LEAF_SIZE - 1, offset:ofst},
            ]);
            // LEAF_SIZE
            assert_eq!(Vec::from_iter(LeafInfoIter::new(LEAF_SIZE, ofst)),  vec![
                LeafInfo{index:0, size:LEAF_SIZE, offset:ofst},
            ]);
            // LEAF_SIZE + 1
            assert_eq!(Vec::from_iter(LeafInfoIter::new(LEAF_SIZE + 1, ofst)),  vec![
                LeafInfo{index:0, size:LEAF_SIZE, offset:ofst},
                LeafInfo{index:1, size:1, offset:ofst + LEAF_SIZE},
            ]);
            // 2 * LEAF_SIZE - 1
            assert_eq!(Vec::from_iter(LeafInfoIter::new(2 * LEAF_SIZE - 1, ofst)),  vec![
                LeafInfo{index:0, size:LEAF_SIZE, offset:ofst},
                LeafInfo{index:1, size:LEAF_SIZE - 1, offset:ofst + LEAF_SIZE},
            ]);
            // 2 * LEAF_SIZE
            assert_eq!(Vec::from_iter(LeafInfoIter::new(2 * LEAF_SIZE, ofst)),  vec![
                LeafInfo{index:0, size:LEAF_SIZE, offset:ofst},
                LeafInfo{index:1, size:LEAF_SIZE, offset:ofst + LEAF_SIZE},
            ]);
            // 2 * LEAF_SIZE + 1
            assert_eq!(Vec::from_iter(LeafInfoIter::new(2 * LEAF_SIZE + 1, ofst)),  vec![
                LeafInfo{index:0, size:LEAF_SIZE, offset:ofst},
                LeafInfo{index:1, size:LEAF_SIZE, offset:ofst + LEAF_SIZE},
                LeafInfo{index:2, size:1, offset:ofst + 2 * LEAF_SIZE},
            ]);
            // 3 * LEAF_SIZE - 1
            assert_eq!(Vec::from_iter(LeafInfoIter::new(3 * LEAF_SIZE - 1, ofst)),  vec![
                LeafInfo{index:0, size:LEAF_SIZE, offset:ofst},
                LeafInfo{index:1, size:LEAF_SIZE, offset:ofst + LEAF_SIZE},
                LeafInfo{index:2, size:LEAF_SIZE - 1, offset:ofst + 2 * LEAF_SIZE},
            ]);
            // 3 * LEAF_SIZE
            assert_eq!(Vec::from_iter(LeafInfoIter::new(3 * LEAF_SIZE, ofst)),  vec![
                LeafInfo{index:0, size:LEAF_SIZE, offset:ofst},
                LeafInfo{index:1, size:LEAF_SIZE, offset:ofst + LEAF_SIZE},
                LeafInfo{index:2, size:LEAF_SIZE, offset:ofst + 2 * LEAF_SIZE},
            ]);
        }
    }
}
