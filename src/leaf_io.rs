//! Leaf-wise File IO.
//!
//! In general, anything that uses LEAF_SIZE should be here.

use std::io;
use std::io::prelude::*;
use std::os::unix::fs::FileExt;
use std::fs::File;
use std::cmp;
use std::path::PathBuf;

use crate::base::*;
use crate::protocol::{hash_leaf, LeafInfo, hash_root, RootInfo, hash_leaf_into, hash_root_raw};


pub fn new_leaf_buf() -> Vec<u8> {
    let mut buf = Vec::with_capacity(LEAF_SIZE as usize);
    buf.resize(LEAF_SIZE as usize, 0);
    buf
}


pub struct TubTop {
    index: u64,
    total: u64,
    buf: Vec<u8>,
}

impl TubTop {
    pub fn new() -> Self {
        let mut buf = Vec::with_capacity(HEADER_LEN + TUB_HASH_LEN);
        buf.resize(HEADER_LEN, 0);
        Self {index: 0, total: 0, buf: buf}
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_buf(self) -> Vec<u8> {
        self.buf
    }

    pub fn hash(&self) -> TubHash {
        self.buf[0..TUB_HASH_LEN].try_into().expect("oops")
    }

    pub fn size(&self) -> u64 {
        u64::from_le_bytes(
            self.buf[TUB_HASH_LEN..HEADER_LEN].try_into().expect("oops")
        )
    }

    pub fn leaf_hash(&self, index: usize) -> TubHash {
        assert_eq!(self.size(), 0);
        let start = HEADER_LEN + (index * TUB_HASH_LEN);
        let stop = start + TUB_HASH_LEN;
        self.buf[start..stop].try_into().expect("oops")
    }

    pub fn hash_next_leaf(&mut self, data: &[u8]) {
        assert!(data.len() > 0 && data.len() <= LEAF_SIZE as usize);
        self.buf.resize(self.buf.len() + TUB_HASH_LEN, 0);
        let start = self.buf.len() - TUB_HASH_LEN;
        hash_leaf_into(self.index, data, &mut self.buf[start..]);
        self.index += 1;
        self.total += data.len() as u64;
    }

    pub fn finalize(&mut self) {
        assert_eq!(self.size(), 0);
        self.buf.splice(TUB_HASH_LEN..HEADER_LEN, self.total.to_le_bytes());
        let hash = hash_root_raw(&self.buf[TUB_HASH_LEN..]);
        self.buf.splice(0..TUB_HASH_LEN, hash);
    }

    pub fn is_large(&self) -> bool {
        assert_ne!(self.size(), 0);
        self.size() > LEAF_SIZE
    }

    pub fn is_small(&self) -> bool {
        ! self.is_large()
    }
}


#[derive(Debug, PartialEq)]
pub struct LeafOffset {
    pub index: u64,
    pub size: u64,
    pub offset: u64,
}

impl LeafOffset {
    pub fn new(index: u64, size: u64, offset: u64) -> Self
    {
        Self {index: index, size: size, offset: offset}
    }
}


#[derive(Debug)]
pub struct LeafOffsetIter {
    pub size: u64,
    pub offset: u64,
    index: u64,
}

impl LeafOffsetIter {
    pub fn new(size: u64, offset: u64) -> Self
    {
        Self {size: size, offset: offset, index: 0}
    }
}

impl Iterator for LeafOffsetIter {
    type Item = LeafOffset;

    fn next(&mut self) -> Option<Self::Item> {
        let consumed = self.index * LEAF_SIZE;
        if consumed < self.size {
            let offset = self.offset + consumed;
            let remaining = self.size - consumed;
            let size = cmp::min(remaining, LEAF_SIZE);
            let info = LeafOffset::new(self.index, size, offset);
            self.index += 1;
            Some(info)
        }
        else {
            None
        }
    }
}


pub struct LeafReader {
    file: File,
    closed: bool,
    index: u64,
    size: u64,
    leaf_hashes: TubHashList,
    tt: TubTop,
}

impl LeafReader {
    pub fn new(file: File) -> Self
    {
        Self {
            file: file,
            closed: false,
            size: 0,
            index: 0,
            leaf_hashes: Vec::new(),
            tt: TubTop::new(),
        }
    }

    pub fn read_next_leaf(&mut self, buf: &mut Vec<u8>) -> io::Result<Option<LeafInfo>>
    {
        if self.closed {
            panic!("LeafReader.read_next_leaf() called after closed");
        }
        buf.resize(LEAF_SIZE as usize, 0);
        let amount = self.file.read(buf)?;
        assert!(amount as u64 <= LEAF_SIZE);
        buf.resize(amount, 0);
        if amount == 0 {
            self.closed = true;
            Ok(None)
        }
        else {
            let info = hash_leaf(self.index, buf);
            self.tt.hash_next_leaf(buf);
            self.size += amount as u64;
            self.leaf_hashes.push(info.hash);
            self.index += 1;
            Ok(Some(info))
        }
    }

    pub fn hash_root(self) -> RootInfo
    {
        if !self.closed {
            panic!("LeafReader.hash_root() called before closed");
        }
        hash_root(self.size, self.leaf_hashes)
    }
}


#[derive(Debug)]
pub struct TmpObject {
    pub id: TubId,
    pub path: PathBuf,
    buf: Option<Vec<u8>>,
    file: Option<File>,
}

impl TmpObject {
    pub fn new(id: TubId, path: PathBuf) -> io::Result<Self>
    {
        Ok(TmpObject {
            id: id,
            path: path,
            buf: None,
            file: None,
        })
    }

    pub fn is_small(&mut self) -> bool
    {
        !self.buf.is_none()
    }

    pub fn into_data(self) -> Vec<u8> {
        self.buf.unwrap()
    }

    pub fn write_leaf(&mut self, buf: &[u8]) -> io::Result<()>
    {
        if self.buf.is_none() && self.file.is_none() {
            // First leaf, keep in memory in case it's a small object
            self.buf = Some(Vec::from(buf));
            Ok(())
        }
        else {
            if self.file.is_none() {
                let mut file = File::options()
                    .create_new(true)
                    .append(true).open(&self.path)?;
                file.write_all(self.buf.as_ref().unwrap())?;
                self.buf = None;
                self.file = Some(file);
            }
            self.file.as_ref().unwrap().write_all(buf)
        }
    }
}

/// Represents an object open for reading (both large and small objects)
#[derive(Debug)]
pub struct Object {
    file: File,
    loi: LeafOffsetIter,
}

impl Object {
    pub fn new(file: File, size: u64, offset: u64) -> Self {
        Self {
            file: file,
            loi: LeafOffsetIter::new(size, offset),
        }
    }

    pub fn read_next_leaf(&mut self, buf: &mut Vec<u8>) -> io::Result<Option<LeafOffset>>
    {
        if let Some(lo) = self.loi.next() {
            buf.resize(lo.size as usize, 0);
            self.file.read_exact_at(buf, lo.offset)?;
            Ok(Some(lo))
        }
        else {
            Ok(None)
        }
    }

    pub fn write_to_file(&mut self, file: &mut File) -> io::Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        while let Some(_) = self.read_next_leaf(&mut buf)? {
            file.write_all(&buf)?;
        }
        Ok(())
    }

}


pub fn hash_object(file: File) -> io::Result<RootInfo>
{
    let mut reader = LeafReader::new(file);
    let mut buf = new_leaf_buf();
    while let Some(_info) = reader.read_next_leaf(&mut buf)? {
        //eprintln!("leaf {}", info.index);
    }
    Ok(reader.hash_root())
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_leaf_buf() {
        let buf = new_leaf_buf();
        assert_eq!(buf.len(), LEAF_SIZE as usize);
        assert_eq!(buf.capacity(), LEAF_SIZE as usize);
        //let s = &mut buf[0..111];
    }

    #[test]
    fn test_leaf_offset_iter() {
        // 0
        assert_eq!(Vec::from_iter(LeafOffsetIter::new(0, 0)), vec![]);
        // 1
        assert_eq!(Vec::from_iter(LeafOffsetIter::new(1, 0)), vec![
            LeafOffset{index:0, size:1, offset:0},
        ]);
        // LEAF_SIZE - 1
        assert_eq!(Vec::from_iter(LeafOffsetIter::new(LEAF_SIZE - 1, 0)),  vec![
            LeafOffset{index:0, size:LEAF_SIZE - 1, offset:0},
        ]);
        // LEAF_SIZE
        assert_eq!(Vec::from_iter(LeafOffsetIter::new(LEAF_SIZE, 0)),  vec![
            LeafOffset{index:0, size:LEAF_SIZE, offset:0},
        ]);
        // LEAF_SIZE + 1
        assert_eq!(Vec::from_iter(LeafOffsetIter::new(LEAF_SIZE + 1, 0)),  vec![
            LeafOffset{index:0, size:LEAF_SIZE, offset:0},
            LeafOffset{index:1, size:1, offset:LEAF_SIZE},
        ]);
        // 2 * LEAF_SIZE - 1
        assert_eq!(Vec::from_iter(LeafOffsetIter::new(2 * LEAF_SIZE - 1, 0)),  vec![
            LeafOffset{index:0, size:LEAF_SIZE, offset:0},
            LeafOffset{index:1, size:LEAF_SIZE - 1, offset:LEAF_SIZE},
        ]);
        // 2 * LEAF_SIZE
        assert_eq!(Vec::from_iter(LeafOffsetIter::new(2 * LEAF_SIZE, 0)),  vec![
            LeafOffset{index:0, size:LEAF_SIZE, offset:0},
            LeafOffset{index:1, size:LEAF_SIZE, offset:LEAF_SIZE},
        ]);
        // 2 * LEAF_SIZE + 1
        assert_eq!(Vec::from_iter(LeafOffsetIter::new(2 * LEAF_SIZE + 1, 0)),  vec![
            LeafOffset{index:0, size:LEAF_SIZE, offset:0},
            LeafOffset{index:1, size:LEAF_SIZE, offset:LEAF_SIZE},
            LeafOffset{index:2, size:1, offset:2 * LEAF_SIZE},
        ]);
        // 3 * LEAF_SIZE - 1
        assert_eq!(Vec::from_iter(LeafOffsetIter::new(3 * LEAF_SIZE - 1, 0)),  vec![
            LeafOffset{index:0, size:LEAF_SIZE, offset:0},
            LeafOffset{index:1, size:LEAF_SIZE, offset:LEAF_SIZE},
            LeafOffset{index:2, size:LEAF_SIZE - 1, offset:2 * LEAF_SIZE},
        ]);
        // 3 * LEAF_SIZE
        assert_eq!(Vec::from_iter(LeafOffsetIter::new(3 * LEAF_SIZE, 0)),  vec![
            LeafOffset{index:0, size:LEAF_SIZE, offset:0},
            LeafOffset{index:1, size:LEAF_SIZE, offset:LEAF_SIZE},
            LeafOffset{index:2, size:LEAF_SIZE, offset:2 * LEAF_SIZE},
        ]);

        for ofst in [0_u64, 1, 2, LEAF_SIZE - 1, LEAF_SIZE, LEAF_SIZE + 1] {
            // 0
            assert_eq!(Vec::from_iter(LeafOffsetIter::new(0, ofst)), vec![]);
            // 1
            assert_eq!(Vec::from_iter(LeafOffsetIter::new(1, ofst)), vec![
                LeafOffset{index:0, size:1, offset:ofst},
            ]);
            // LEAF_SIZE - 1
            assert_eq!(Vec::from_iter(LeafOffsetIter::new(LEAF_SIZE - 1, ofst)),  vec![
                LeafOffset{index:0, size:LEAF_SIZE - 1, offset:ofst},
            ]);
            // LEAF_SIZE
            assert_eq!(Vec::from_iter(LeafOffsetIter::new(LEAF_SIZE, ofst)),  vec![
                LeafOffset{index:0, size:LEAF_SIZE, offset:ofst},
            ]);
            // LEAF_SIZE + 1
            assert_eq!(Vec::from_iter(LeafOffsetIter::new(LEAF_SIZE + 1, ofst)),  vec![
                LeafOffset{index:0, size:LEAF_SIZE, offset:ofst},
                LeafOffset{index:1, size:1, offset:ofst + LEAF_SIZE},
            ]);
            // 2 * LEAF_SIZE - 1
            assert_eq!(Vec::from_iter(LeafOffsetIter::new(2 * LEAF_SIZE - 1, ofst)),  vec![
                LeafOffset{index:0, size:LEAF_SIZE, offset:ofst},
                LeafOffset{index:1, size:LEAF_SIZE - 1, offset:ofst + LEAF_SIZE},
            ]);
            // 2 * LEAF_SIZE
            assert_eq!(Vec::from_iter(LeafOffsetIter::new(2 * LEAF_SIZE, ofst)),  vec![
                LeafOffset{index:0, size:LEAF_SIZE, offset:ofst},
                LeafOffset{index:1, size:LEAF_SIZE, offset:ofst + LEAF_SIZE},
            ]);
            // 2 * LEAF_SIZE + 1
            assert_eq!(Vec::from_iter(LeafOffsetIter::new(2 * LEAF_SIZE + 1, ofst)),  vec![
                LeafOffset{index:0, size:LEAF_SIZE, offset:ofst},
                LeafOffset{index:1, size:LEAF_SIZE, offset:ofst + LEAF_SIZE},
                LeafOffset{index:2, size:1, offset:ofst + 2 * LEAF_SIZE},
            ]);
            // 3 * LEAF_SIZE - 1
            assert_eq!(Vec::from_iter(LeafOffsetIter::new(3 * LEAF_SIZE - 1, ofst)),  vec![
                LeafOffset{index:0, size:LEAF_SIZE, offset:ofst},
                LeafOffset{index:1, size:LEAF_SIZE, offset:ofst + LEAF_SIZE},
                LeafOffset{index:2, size:LEAF_SIZE - 1, offset:ofst + 2 * LEAF_SIZE},
            ]);
            // 3 * LEAF_SIZE
            assert_eq!(Vec::from_iter(LeafOffsetIter::new(3 * LEAF_SIZE, ofst)),  vec![
                LeafOffset{index:0, size:LEAF_SIZE, offset:ofst},
                LeafOffset{index:1, size:LEAF_SIZE, offset:ofst + LEAF_SIZE},
                LeafOffset{index:2, size:LEAF_SIZE, offset:ofst + 2 * LEAF_SIZE},
            ]);
        }
    }
}
