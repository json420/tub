//! Leaf-wise File IO.
//!
//! In general, anything that uses LEAF_SIZE should be here.

use std::io;
use std::io::prelude::*;
use std::os::unix::fs::FileExt;
use std::fs;
use std::fs::File;
use std::cmp;
use std::fmt;
use std::path::PathBuf;

use crate::base::*;
use crate::dbase32::db32enc_str;
use crate::protocol::{hash_leaf, hash_root, hash_tombstone};


pub fn new_leaf_buf() -> Vec<u8> {
    let mut buf = Vec::with_capacity(LEAF_SIZE as usize);
    buf.resize(LEAF_SIZE as usize, 0);
    buf
}


pub fn hash_file(file: File) -> io::Result<TubTop>
{
    let mut reader = LeafReader::new(file, 0);
    let mut buf = new_leaf_buf();
    while let Some(_info) = reader.read_next_leaf(&mut buf)? {
        //eprintln!("leaf {}", info.index);
    }
    Ok(reader.finalize())
}


// FIXME: not sure this is useful enough to keep around
#[derive(Debug, PartialEq)]
pub struct LeafInfo {
    pub hash: TubHash,
    pub index: u64,
}

impl LeafInfo {
    pub fn new(hash: TubHash, index: u64) -> Self {
        Self {hash: hash, index: index}
    }
}


pub fn get_leaf_count(size: u64) -> u64 {
    //assert!(size > 0);
    let count = size / LEAF_SIZE;
    if size % LEAF_SIZE == 0 {
        count
    }
    else {
        count + 1
    }
}

/// Returns size of the root hash + u64 + leaf_hashes.
pub fn get_preamble_size(size: u64) -> u64 {
    (HEADER_LEN as u64) + get_leaf_count(size) * (TUB_HASH_LEN as u64)
}


/// Returns size of header + leaf_hashes + data.
pub fn get_full_object_size(size: u64) -> u64 {
    get_preamble_size(size) + size
}


pub fn get_buffer_size(size: u64) -> u64 {
    get_preamble_size(size) + cmp::min(size, LEAF_SIZE as u64)
}

pub fn get_leaf_range(index: u64, size: u64) -> Option<(u64, u64)> {
    //assert_ne!(size, 0);  // Should we panic on size==0 case?
    let start = index * LEAF_SIZE;
    if start < size {
        let stop = cmp::min(start + LEAF_SIZE, size);
        Some((start, stop))
    }
    else {
        None
    }
}

pub fn get_leaf_size(index: u64, size: u64) -> Option<u64> {
    if let Some((start, stop)) = get_leaf_range(index, size) {
        Some(stop - start)
    }
    else {
        None
    }
}



#[derive(Debug)]
pub struct TubTop {
    index: u64,
    total: u64,
    buf: Vec<u8>,
}

impl TubTop {
    pub fn new() -> Self {
        Self::new_with_buf(Vec::with_capacity(HEAD_LEN))
    }

    pub fn new_with_buf(mut buf: Vec<u8>) -> Self {
        buf.clear();
        buf.resize(HEAD_LEN, 0);
        Self {index: 0, total: 0, buf: buf}
    }

    pub fn new_for_leaf_buf() -> Self {
        let size = HEAD_LEN + 16 * TUB_HASH_LEN + (LEAF_SIZE as usize);
        let mut buf = Vec::with_capacity(size);
        buf.resize(HEAD_LEN, 0);
        Self {index: 0, total: 0, buf: buf}
    }

    pub fn into_buf(self) -> Vec<u8> {
        self.buf
    }

    pub fn reset(&mut self) {
        self.index = 0;
        self.total = 0;
        self.buf.clear();
        self.buf.resize(HEAD_LEN, 0);
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn hash(&self) -> TubHash {
        self.buf[0..TUB_HASH_LEN].try_into().expect("oops")
    }

    fn set_hash(&mut self, hash: &TubHash) {
        self.buf[0..TUB_HASH_LEN].copy_from_slice(hash);
    }

    pub fn size(&self) -> u64 {
        u64::from_le_bytes(
            self.buf[TUB_HASH_LEN..HEADER_LEN].try_into().expect("oops")
        )
    }

    fn set_size(&mut self, size: u64) {
        self.buf[TUB_HASH_LEN..HEADER_LEN].copy_from_slice(&size.to_le_bytes());
    }

    pub fn leaf_hash(&self, index: usize) -> TubHash {
        let start = HEADER_LEN + (index * TUB_HASH_LEN);
        let stop = start + TUB_HASH_LEN;
        self.buf[start..stop].try_into().expect("oops")
    }

    fn set_leaf_hash(&mut self, index: usize, hash: &TubHash) {
        let start = HEADER_LEN + (index * TUB_HASH_LEN);
        let stop = start + TUB_HASH_LEN;
        self.buf[start..stop].copy_from_slice(hash);
    }

    pub fn is_large(&self) -> bool {
        self.size() > LEAF_SIZE
    }

    pub fn is_small(&self) -> bool {
        ! self.is_large()
    }

    pub fn is_tombstone(&self) -> bool {
        self.size() == 0 && self.leaf_hash(0) == hash_tombstone(&self.hash())
    }

    fn compute_root(&self) -> TubHash {
        hash_root(self.size(), &self.as_leaf_hashes())
    }

    pub fn is_valid(&self) -> bool {
        self.size() > 0 && self.hash() == self.compute_root()
    }

    pub fn has_valid_data(&self) -> bool {
        self.len() == get_full_object_size(self.size()) as usize
        && self.leaf_hash(0) == hash_leaf(0, self.as_data())
    }

    pub fn is_valid_for_copy(&self) -> bool {
        self.is_valid() && (self.is_large() || self.has_valid_data())
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_leaf_hashes(&self) -> &[u8] {
        assert_ne!(self.size(), 0);
        let stop = get_preamble_size(self.size()) as usize;
        &self.buf[HEADER_LEN..stop]
    }

    pub fn as_data(&self) -> &[u8] {
        let size = self.size();
        let start = get_preamble_size(size);
        let stop = start + size;
        assert_eq!(stop, get_full_object_size(size));
        &self.buf[start as usize..stop as usize]
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn as_mut_head(&mut self) -> &mut [u8] {
        &mut self.buf[0..HEAD_LEN]
    }

    pub fn as_mut_tail(&mut self) -> &mut [u8] {
        &mut self.buf[HEAD_LEN..]
    }

    pub fn as_mut_data(&mut self) -> &mut [u8] {
        let size = self.size();
        let start = get_preamble_size(size);
        let stop = start + size;
        assert_eq!(stop, get_full_object_size(size));
        &mut self.buf[start as usize..stop as usize]
    }

    pub fn resize_to_claimed_size(&mut self) {
        let size = get_preamble_size(self.size()) as usize;
        self.buf.resize(size, 0);
    }

    pub fn resize_for_size(&mut self, size: u64) {
        self.buf.resize(get_preamble_size(size) as usize, 0);
    }

    pub fn resize_for_size_plus_data(&mut self, size: u64) {
        assert!(size > 0);
        assert!(size <= LEAF_SIZE);
        self.buf.resize(get_full_object_size(size) as usize, 0);
    }

    pub fn resize_for_copy(&mut self, size: u64) {
        if size <= LEAF_SIZE {
            self.resize_for_size_plus_data(size);
        }
        else {
            self.resize_for_size(size);
        }
    }

    pub fn resize_for_leaf_buf(&mut self, size: u64) {
        assert!(size > 0);
        self.total = size;
        self.buf.resize(get_buffer_size(size) as usize, 0);
    }

    pub fn hash_next_leaf_internal(&mut self) {
        //if let Some(size) = get_leaf_size(
    }

    pub fn hash_next_leaf(&mut self, data: &[u8]) -> LeafInfo {
        assert!(data.len() > 0 && data.len() <= LEAF_SIZE as usize);
        if self.index != 0 {
            self.buf.resize(self.buf.len() + TUB_HASH_LEN, 0);
        }
        let hash = hash_leaf(self.index, data);
        self.set_leaf_hash(self.index as usize, &hash);
        let info = LeafInfo::new(hash, self.index);
        self.index += 1;
        self.total += data.len() as u64;
        info
    }

    pub fn hash_data(&mut self, data: &[u8]) -> TubHash {
        assert!(data.len() > 0);
        self.reset();
        for (start, stop) in LeafRangeIter::new(data.len() as u64) {
            self.hash_next_leaf(&data[start as usize..stop as usize]);
        }
        self.finalize()
    }

    pub fn finalize(&mut self) -> TubHash {
        assert!(self.total > 0);
        self.set_size(self.total);
        self.finalize_raw()
    }

    pub fn finalize_raw(&mut self) -> TubHash {
        assert!(self.size() > 0);
        let hash = self.compute_root();
        self.set_hash(&hash);
        hash
    }

}

impl fmt::Display for TubTop {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{}", db32enc_str(&self.hash()))
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

#[derive(Debug)]
pub struct LeafRangeIter {
    pub size: u64,
    index: u64,
}

impl LeafRangeIter {
    pub fn new(size: u64) -> Self
    {
        Self {size: size, index: 0}
    }
}

impl Iterator for LeafRangeIter {
    type Item = (u64, u64);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(r) = get_leaf_range(self.index, self.size) {
            self.index += 1;
            Some(r)
        }
        else {
            None
        }
    }
}


#[derive(Debug)]
pub struct LeafReader {
    file: File,
    tt: TubTop,
    size: u64,
    closed: bool,
}

impl LeafReader {
    pub fn new(file: File, size: u64) -> Self
    {
        Self::new_with_tubtop(file, size, TubTop::new())
    }

    pub fn new_with_tubtop(file: File, size: u64, tt: TubTop) -> Self {
        Self {file: file, tt: tt, size: size, closed: false}
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
            let info = self.tt.hash_next_leaf(buf);
            Ok(Some(info))
        }
    }

    pub fn read_next_internal(&mut self) -> io::Result<Option<&[u8]>> {
        Ok(Some(self.tt.as_data()))
    }

    pub fn finalize(mut self) -> TubTop {
        if !self.closed {
            panic!("LeafReader.finalize() called before closed");
        }
        self.tt.finalize();
        self.tt
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

    pub fn is_small(&self) -> bool
    {
        !self.buf.is_none()
    }

    pub fn into_data(self) -> Vec<u8> {
        self.buf.unwrap()
    }

    pub fn remove_file(&mut self) -> io::Result<()> {
        if self.file.is_some() {
            assert!(! self.is_small());
            eprintln!("Removing temporary file {:?}", &self.path);
            fs::remove_file(&self.path)?;
            self.file = None;
        }
        Ok(())
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

    pub fn write_all(&mut self, buf: &[u8]) -> io::Result<()>
    {
        assert!(self.buf.is_none());
        if self.file.is_none() {
            let file = File::options()
                .create_new(true)
                .append(true).open(&self.path)?;
            self.file = Some(file);
        }
        self.file.as_ref().unwrap().write_all(buf)
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
    fn test_tubtop() {
        let mut tt = TubTop::new();
        assert_eq!(tt.len(), HEAD_LEN);
        assert_eq!(tt.hash(), [0_u8; TUB_HASH_LEN]);
        assert_eq!(tt.size(), 0);
        assert_eq!(tt.is_valid(), false);
        assert_eq!(tt.is_small(), true);
        assert_eq!(tt.is_large(), false);
        assert_eq!(tt.as_buf(), [0_u8; HEAD_LEN]);
        assert_eq!(tt.as_mut_head(), [0_u8; HEAD_LEN]);

        // 1 Leaf
        for size in [1, 2, 3, LEAF_SIZE - 2, LEAF_SIZE - 1, LEAF_SIZE] {
            let mut tt = TubTop::new();

            // Get mutable reference to header portion of buffer
            let head = tt.as_mut_head();
            assert_eq!(head, [0_u8; HEAD_LEN]);

            // Set the size
            head[TUB_HASH_LEN..HEADER_LEN].copy_from_slice(&size.to_le_bytes());
            tt.resize_to_claimed_size();
            assert_eq!(tt.size(), size);
            assert_eq!(tt.hash(), [0_u8; TUB_HASH_LEN]);
            assert_eq!(tt.len(), HEAD_LEN);
            assert_eq!(tt.is_small(), true);
            assert_eq!(tt.is_large(), false);

            // Test validation stuffs
            assert_eq!(tt.is_valid(), false);
            assert_eq!(tt.size(), size);
            tt.finalize_raw();
            assert_eq!(tt.size(), size);
            assert_eq!(tt.is_valid(), true);
        }

        // 2 Leaves
        for size in [LEAF_SIZE + 1, 2 * LEAF_SIZE - 1, 2 * LEAF_SIZE] {
            let mut tt = TubTop::new();

            // Get mutable reference to header portion of buffer
            let head = tt.as_mut_head();
            assert_eq!(head, [0_u8; HEAD_LEN]);

            // Set the size
            head[TUB_HASH_LEN..HEADER_LEN].copy_from_slice(&size.to_le_bytes());
            tt.resize_to_claimed_size();
            assert_eq!(tt.size(), size);
            assert_eq!(tt.hash(), [0_u8; TUB_HASH_LEN]);
            assert_eq!(tt.len(), HEAD_LEN + TUB_HASH_LEN);
            assert_eq!(tt.is_small(), false);
            assert_eq!(tt.is_large(), true);

            // Test validation stuffs
            assert_eq!(tt.is_valid(), false);
            tt.finalize_raw();
            assert_eq!(tt.is_valid(), true);
        }
    }

    #[test]
    fn test_get_leaf_count() {
        assert_eq!(get_leaf_count(0), 0);
        assert_eq!(get_leaf_count(1), 1);
        assert_eq!(get_leaf_count(2), 1);

        assert_eq!(get_leaf_count(LEAF_SIZE - 1), 1);
        assert_eq!(get_leaf_count(LEAF_SIZE), 1);
        assert_eq!(get_leaf_count(LEAF_SIZE + 1), 2);

        assert_eq!(get_leaf_count(2 * LEAF_SIZE - 1), 2);
        assert_eq!(get_leaf_count(2 * LEAF_SIZE), 2);
        assert_eq!(get_leaf_count(2 * LEAF_SIZE + 1), 3);
    }

    #[test]
    fn test_get_preamble_size() {
        let head = HEADER_LEN as u64;
        let tub = TUB_HASH_LEN as u64;
        assert_eq!(get_preamble_size(0), head);

        assert_eq!(get_preamble_size(1), head + tub);
        assert_eq!(get_preamble_size(2), head + tub);
        assert_eq!(get_preamble_size(LEAF_SIZE - 1), head + tub);
        assert_eq!(get_preamble_size(LEAF_SIZE), head + tub);

        assert_eq!(get_preamble_size(LEAF_SIZE + 1), head + tub * 2);
        assert_eq!(get_preamble_size(2 * LEAF_SIZE - 1), head + tub * 2);
        assert_eq!(get_preamble_size(2 * LEAF_SIZE), head + tub * 2);

        assert_eq!(get_preamble_size(2 * LEAF_SIZE + 1), head + tub * 3);
        assert_eq!(get_preamble_size(3 * LEAF_SIZE - 1), head + tub * 3);
        assert_eq!(get_preamble_size(3 * LEAF_SIZE), head + tub * 3);
    }

    #[test]
    fn test_get_full_object_size() {
        assert_eq!(get_full_object_size(1), (HEAD_LEN + 1) as u64);
        assert_eq!(get_full_object_size(2), (HEAD_LEN + 2) as u64);
        assert_eq!(get_full_object_size(3), (HEAD_LEN + 3) as u64);
        assert_eq!(get_full_object_size(LEAF_SIZE), HEAD_LEN as u64 + LEAF_SIZE);
        assert_eq!(get_full_object_size(LEAF_SIZE + 1),
            HEAD_LEN as u64 + TUB_HASH_LEN as u64 + LEAF_SIZE + 1
        );
    }

    #[test]
    fn test_get_buffer_size() {
        assert_eq!(get_buffer_size(1), (HEAD_LEN + 1) as u64);
        assert_eq!(get_buffer_size(2), (HEAD_LEN + 2) as u64);
        assert_eq!(get_buffer_size(3), (HEAD_LEN + 3) as u64);
        assert_eq!(get_buffer_size(LEAF_SIZE), HEAD_LEN as u64 + LEAF_SIZE);
        assert_eq!(get_buffer_size(LEAF_SIZE + 1),
            HEAD_LEN as u64 + TUB_HASH_LEN as u64 + LEAF_SIZE
        );
        assert_eq!(get_buffer_size(LEAF_SIZE + 1),
            HEAD_LEN as u64 + TUB_HASH_LEN as u64 + LEAF_SIZE
        );

        for size in [42 * LEAF_SIZE, 420 * LEAF_SIZE, u64::MAX - 1, u64::MAX] {
            assert_eq!(
                get_buffer_size(size),
                get_preamble_size(size) + LEAF_SIZE
            );
        }
    }

    #[test]
    fn test_get_leaf_range() {
        assert_eq!(get_leaf_range(0, 0), None);
        assert_eq!(get_leaf_range(0, 1), Some((0, 1)));
        assert_eq!(get_leaf_range(0, LEAF_SIZE - 1), Some((0, LEAF_SIZE - 1)));
        assert_eq!(get_leaf_range(0, LEAF_SIZE), Some((0, LEAF_SIZE)));
        assert_eq!(get_leaf_range(0, LEAF_SIZE + 1), Some((0, LEAF_SIZE)));

        assert_eq!(get_leaf_range(1, 0), None);
        assert_eq!(get_leaf_range(1, 1), None);
        assert_eq!(get_leaf_range(1, LEAF_SIZE - 1), None);
        assert_eq!(get_leaf_range(1, LEAF_SIZE), None);
        assert_eq!(get_leaf_range(1, LEAF_SIZE + 1), Some((LEAF_SIZE, LEAF_SIZE + 1)));

        assert_eq!(get_leaf_range(2, 0), None);
        assert_eq!(get_leaf_range(2, LEAF_SIZE + 1), None);
        assert_eq!(get_leaf_range(2, 2 * LEAF_SIZE), None);
        assert_eq!(get_leaf_range(2, 2 * LEAF_SIZE), None);
        assert_eq!(get_leaf_range(2, 2 * LEAF_SIZE + 1),
            Some((2 * LEAF_SIZE, 2 * LEAF_SIZE + 1))
        );
    }

    #[test]
    fn test_leaf_range_iter() {
        assert_eq!(Vec::from_iter(LeafRangeIter::new(0)), vec![]);
        assert_eq!(Vec::from_iter(LeafRangeIter::new(1)), vec![(0, 1)]);
        assert_eq!(Vec::from_iter(LeafRangeIter::new(2)), vec![(0, 2)]);
        assert_eq!(Vec::from_iter(LeafRangeIter::new(LEAF_SIZE - 1)),
            vec![(0, LEAF_SIZE - 1)]
        );
        assert_eq!(Vec::from_iter(LeafRangeIter::new(LEAF_SIZE)),
            vec![(0, LEAF_SIZE)]
        );
        assert_eq!(Vec::from_iter(LeafRangeIter::new(LEAF_SIZE + 1)),
            vec![(0, LEAF_SIZE), (LEAF_SIZE, LEAF_SIZE + 1)]
        );
        assert_eq!(Vec::from_iter(LeafRangeIter::new(2* LEAF_SIZE - 1)),
            vec![(0, LEAF_SIZE), (LEAF_SIZE, 2* LEAF_SIZE - 1)]
        );
        assert_eq!(Vec::from_iter(LeafRangeIter::new(2* LEAF_SIZE)),
            vec![(0, LEAF_SIZE), (LEAF_SIZE, 2* LEAF_SIZE)]
        );
        assert_eq!(Vec::from_iter(LeafRangeIter::new(2* LEAF_SIZE + 1)), vec![
            (0, LEAF_SIZE),
            (LEAF_SIZE, 2* LEAF_SIZE),
            (2 * LEAF_SIZE, 2 * LEAF_SIZE + 1),
        ]);
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
