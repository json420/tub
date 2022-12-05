//! Leaf-wise File IO.
//!
//! In general, anything that uses LEAF_SIZE should be here.

use std::io;
use std::io::prelude::*;
use std::os::unix::fs::FileExt;
use std::fs::File;
use std::cmp;
use std::fmt;
use std::path::PathBuf;
use std::ops;

use crate::base::*;
use crate::dbase32::db32enc_str;
use crate::protocol::{hash_leaf, hash_root, hash_tombstone,  hash_payload};



pub fn hash_file(file: File, size: u64) -> io::Result<TubBuf>
{
    let mut tbuf = TubBuf::new();
    tbuf.resize(size);
    let mut reader = LeafReader::new(tbuf, file);
    while let Some(_buf) = reader.read_next_leaf()? {
        // No need to write buf anywhere
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

pub fn get_leaf_payload_size(size: u64) -> u64 {
    TUB_HASH_LEN as u64 * get_leaf_count(size)
}

/// Returns size of the root hash + u64 + leaf_hashes.
pub fn get_preamble_size(size: u64) -> u64 {
    (TUB_HASH_LEN as u64 + 9) + get_leaf_count(size) * (TUB_HASH_LEN as u64)
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


#[derive(Debug, PartialEq, Clone, Copy)]
struct LeafState {
    closed: bool,
    object_size: u64,
    leaf_index: u64,
    file_start: u64,
    file_stop: u64,
    leaf_start: usize,
    leaf_stop: usize,
    leaf_hash_start: usize,
    leaf_hash_stop: usize,
}

impl LeafState {
    fn new_raw(object_size: u64, leaf_index: u64) -> Self {
        if object_size == 0 {
            Self {
                closed: true,
                object_size: 0,
                leaf_index: 0,
                file_start: 0,
                file_stop: 0,
                leaf_start: 0,
                leaf_stop: 0,
                leaf_hash_start: 0,
                leaf_hash_stop: 0,
            }
        }
        else if object_size <= LEAF_SIZE {
            let closed = leaf_index > 0;
            let leaf_index = 0;
            let file_start = 0;
            let file_stop = object_size;
            let leaf_start = HEADER_LEN;
            let leaf_stop = leaf_start + object_size as usize;
            let leaf_hash_start = HEADER_LEN;
            let leaf_hash_stop = HEADER_LEN;
            Self {
                closed: closed,
                object_size: object_size,
                leaf_index: leaf_index,
                file_start: file_start,
                file_stop: file_stop,
                leaf_start: leaf_start,
                leaf_stop: leaf_stop,
                leaf_hash_start: leaf_hash_start,
                leaf_hash_stop: leaf_hash_stop,
            }
        }
        else {
            let count = get_leaf_count(object_size);
            assert!(count >= 2);
            let closed = if leaf_index < count {false} else {true};
            let leaf_index = if closed {count - 1} else {leaf_index};
            assert!(leaf_index < count);
            let file_start = leaf_index * LEAF_SIZE;
            let file_stop = cmp::min(file_start + LEAF_SIZE, object_size);
            let leaf_start = HEADER_LEN + TUB_HASH_LEN * count as usize;
            let leaf_stop = leaf_start + (file_stop - file_start) as usize;
            let leaf_hash_start = HEADER_LEN + leaf_index as usize * TUB_HASH_LEN;
            let leaf_hash_stop = leaf_hash_start + TUB_HASH_LEN;
            Self {
                closed: closed,
                object_size: object_size,
                leaf_index: leaf_index,
                file_start: file_start,
                file_stop: file_stop,
                leaf_start: leaf_start,
                leaf_stop: leaf_stop,
                leaf_hash_start: leaf_hash_start,
                leaf_hash_stop: leaf_hash_stop,
            }
        }
    }

    fn new(object_size: u64) -> Self {
        Self::new_raw(object_size, 0)
    }

    fn next_leaf(self) -> Self {
        Self::new_raw(self.object_size, self.leaf_index + 1)
    }

    fn is_small(&self) -> bool {
        self.object_size > 0 && self.object_size <= LEAF_SIZE
    }

    fn is_large(&self) -> bool {
        self.object_size > 0 && ! self.is_small()
    }

    fn can_read(&self) -> bool {
        self.object_size > 0
    }

    fn can_write(&self) -> bool {
        self.object_size > 0 && ! self.closed
    }

    fn check_can_read(&self) {
        if self.object_size == 0 {
            panic!("Cannot read from TubBuf when object_size is zero");
        }
        assert!(self.can_read());
    }

    fn check_can_write(&self) {
        if self.object_size == 0 {
            panic!("Cannot write to TubBuf when object_size is zero");
        }
        if self.closed {
            panic!("Cannot write to TubBuf when closed");
        }
        assert!(self.can_write());
    }

    fn leaf_hash_range(&self) -> ops::Range<usize> {
        self.leaf_hash_start..self.leaf_hash_stop
    }

    fn leaf_hashes_range(&self) -> ops::Range<usize> {
        HEADER_LEN..self.leaf_start
    }

    fn leaf_range(&self) -> ops::Range<usize> {
        self.leaf_start..self.leaf_stop
    }

    fn payload_range(&self) -> ops::Range<usize> {
        let start = HEADER_LEN;
        let stop = if self.is_small() {self.leaf_stop} else {self.leaf_start};
        start..stop
    }

    fn commit_range(&self) -> ops::Range<usize> {
        let stop = if self.is_small() {
            self.leaf_stop
        } else {
            self.leaf_start
        };
        0..stop
    }
    
}

const PREALLOC_COUNT: usize = 1024;
const PREALLOC_LEN: usize = HEADER_LEN + (PREALLOC_COUNT * TUB_HASH_LEN) + LEAF_SIZE as usize;


#[derive(Debug)]
pub struct TubBuf {
    buf: Vec<u8>,
    state: LeafState,
}


// When state.closed == true, don't allow access to mutable buffers
// When state.size == 0, also don't allow access to read-only buffers
impl TubBuf {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(PREALLOC_LEN),
            state: LeafState::new(0),
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn resize(&mut self, size: u64) {
        self.state = LeafState::new(size);
        self.buf.clear();
        self.buf.resize(self.state.leaf_stop, 0);
    }

    fn compute_leaf(&self) -> TubHash {
        hash_leaf(self.state.leaf_index, self.as_leaf())
    }

    fn compute_payload(&self) -> TubHash {
        hash_payload(self.state.object_size, self.as_payload())
    }

    fn compute_root(&self) -> TubHash {
        hash_root(self.state.object_size, &self.payload_hash())
    }

    fn compute_tombstone(&self) -> TubHash {
        hash_tombstone(&self.hash())
    }

    pub fn hash(&self) -> TubHash {
        self.buf[ROOT_HASH_RANGE].try_into().expect("oops")
    }

    fn set_hash(&mut self, hash: &TubHash) {
        self.buf[ROOT_HASH_RANGE].copy_from_slice(hash);
    }

    pub fn payload_hash(&self) -> TubHash {
        self.buf[PAYLOAD_HASH_RANGE].try_into().expect("oops")
    }

    fn set_payload_hash(&mut self, hash: &TubHash) {
        self.buf[PAYLOAD_HASH_RANGE].copy_from_slice(hash);
    }

    pub fn size(&self) -> u64 {
        u64::from_le_bytes(
            self.buf[SIZE_RANGE].try_into().expect("oops")
        )

    }

    fn set_size(&mut self, size: u64) {
        self.buf[SIZE_RANGE].copy_from_slice(&size.to_le_bytes());
    }

    pub fn hash_leaf(&mut self) {
        self.state.check_can_read();
        let hash = self.compute_leaf();
        self.buf[self.state.leaf_hash_range()].copy_from_slice(&hash);
        self.state = self.state.next_leaf();
    }

    pub fn hash_payload(&mut self) {
        let hash = self.compute_payload();
        self.set_payload_hash(&hash);
    }

    pub fn as_commit(&self) -> &[u8] {
        self.state.check_can_read();
        &self.buf[self.state.commit_range()]
    }

    pub fn as_payload(&self) -> &[u8] {
        &self.buf[self.state.payload_range()]
    }

    pub fn as_leaf_hash(&self) -> &[u8] {
        // FIXME: This is probably dumb
        if self.state.object_size == 0 {
            &self.buf[PAYLOAD_HASH_RANGE]
        }
        else {
            &self.buf[self.state.leaf_hash_range()]
        }
    }

    pub fn as_leaf_hashes(&self) -> &[u8] {
        self.state.check_can_read();
        &self.buf[self.state.leaf_hashes_range()]
    }

    pub fn as_leaf(&self) -> &[u8] {
        self.state.check_can_read();
        &self.buf[self.state.leaf_range()]
    }

    pub fn as_mut_commit(&mut self) -> &mut [u8] {
        self.state.check_can_write();
        &mut self.buf[self.state.commit_range()]
    }

    pub fn as_mut_leaf(&mut self) -> Option<&mut [u8]> {
        if self.state.can_write() {
            self.state.check_can_write();  // Rendundant, but double check for now
            Some(&mut self.buf[self.state.leaf_range()])
        }
        else {
            None
        }
    }

    pub fn hash_data(&mut self, data: &[u8]) -> TubHash {
        if self.is_large() {
            panic!("FIXME: large objects not yet supported");
        }
        self.resize(data.len() as u64);
        self.buf[self.state.leaf_range()].copy_from_slice(data);
        self.finalize();
        self.hash()
    }

    pub fn is_tombstone(&self) -> bool {
        assert_eq!(self.len(), HEADER_LEN); 
        self.size() == 0 && self.as_leaf_hash() == self.compute_tombstone()
    }

    pub fn is_small(&self) -> bool {
        self.state.is_small()
    }

    pub fn is_large(&self) -> bool {
        self.state.is_large()
    }

    pub fn is_valid_for_commit(&self) -> bool {
        self.size() == self.state.object_size
        && self.payload_hash() == self.compute_payload()
        && self.hash() == self.compute_root()
    }

    pub fn finalize(&mut self) {
        self.hash_payload();
        self.set_size(self.state.object_size);
        self.set_hash(&self.compute_root());
    }
}


pub struct Header<'a> {
    buf: &'a [u8],
}

impl<'a> Header<'a>
{
    pub fn new(buf: &'a [u8]) ->  Self {
        Self {buf: buf}
    }

    pub fn as_root_hash(&self) -> &[u8] {
        &self.buf[ROOT_HASH_RANGE]
    }

    pub fn as_size(&self) -> &[u8] {
        &self.buf[SIZE_RANGE]
    }

    pub fn as_payload_hash(&self) -> &[u8] {
        &self.buf[ROOT_HASH_RANGE]
    }

    pub fn as_tail(&self) -> &[u8] {
        &self.buf[TAIL_RANGE]
    }
}


pub struct ReindexBuf {
    buf: [u8; HEADER_LEN],
}

impl ReindexBuf {
    pub fn new() -> Self {
        //let mut buf = Vec::with_capacity(HEADER_LEN);
        //buf.resize(HEADER_LEN, 0);
        Self {buf: [0_u8; HEADER_LEN]}
    }

    pub fn is_object(&self) -> bool {
        self.size() != 0 && self.hash() == hash_root(self.size(), &self.payload_hash())
    }

    pub fn is_tombstone(&self) -> bool {
        self.size() == 0 && self.payload_hash() == hash_tombstone(&self.hash())
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8]{
        &mut self.buf
    }

    pub fn size(&self) -> u64 {
        assert_ne!(self.buf.len(), 0);
        u64::from_le_bytes(
            self.buf[SIZE_RANGE].try_into().expect("oops")
        )
    }

    pub fn hash(&self) -> TubHash {
        self.buf[ROOT_HASH_RANGE].try_into().expect("oops")
    }

    pub fn payload_hash(&self) -> TubHash {
        self.buf[PAYLOAD_HASH_RANGE].try_into().expect("oops")
    }

    pub fn offset_size(&self) -> u64 {
        HEADER_LEN as u64 + if self.size() > LEAF_SIZE {
            get_leaf_payload_size(self.size())
        }
        else {
            self.size()
        }
    }

    pub fn reset(&mut self) {
        self.buf.fill(0);
    }
}

impl fmt::Display for TubBuf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", db32enc_str(&self.hash()))
    }
}

impl fmt::Display for ReindexBuf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", db32enc_str(&self.hash()))
    }
}


#[derive(Debug)]
pub struct LeafReader {
    pub tbuf: TubBuf,
    pub file: File,
}

impl LeafReader {
    pub fn new(tbuf: TubBuf, file: File) -> Self {
        Self {tbuf: tbuf, file: file}
    }

    pub fn is_small(&self) -> bool {
        self.tbuf.is_small()
    }

    pub fn is_large(&self) -> bool {
        self.tbuf.is_large()
    }

    pub fn read_in_small(&mut self) -> io::Result<()> {
        assert!(self.tbuf.is_small());
        self.file.read_exact(self.tbuf.as_mut_leaf().unwrap())?;
        Ok(())
    }

    pub fn read_next_leaf(&mut self) -> io::Result<Option<&[u8]>> {
        assert!(self.tbuf.is_large());
        if let Some(buf) = self.tbuf.as_mut_leaf() {
            self.file.read_exact(buf)?;
            self.tbuf.hash_leaf();
            Ok(Some(self.tbuf.as_leaf()))
        }
        else {
            Ok(None)
        }
    }

    pub fn finalize(mut self) -> TubBuf {
        self.tbuf.finalize();
        self.tbuf
    }
}


#[derive(Debug)]
pub struct TmpObject {
    pub id: TubId,
    pub pb: PathBuf,
    file: File,
}

impl TmpObject {
    pub fn new(id: TubId, pb: PathBuf) -> io::Result<Self>
    {
        let file = File::options().append(true).create_new(true).open(&pb)?;
        Ok(TmpObject {id: id, pb: pb, file: file})
    }

    pub fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.file.write_all(buf)
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leafstate() {
        for leaf_index in [0, 1, 2, 3] {
            let state = LeafState::new_raw(0, leaf_index);
            assert_eq!(state, LeafState {
                closed: true,
                object_size: 0,
                leaf_index: 0,
                file_start: 0,
                file_stop: 0,
                leaf_start: 0,
                leaf_stop: 0,
                leaf_hash_start: 0,
                leaf_hash_stop: 0,
            });
            assert!(! state.can_read());
            assert!(! state.can_write());
        }

        for size in [1, 2, 3, LEAF_SIZE - 1, LEAF_SIZE] {
            let state = LeafState::new_raw(size, 0);
            assert_eq!(state, LeafState {
                closed: false,
                object_size: size,
                leaf_index: 0,
                file_start: 0,
                file_stop: size,
                leaf_start: HEADER_LEN,
                leaf_stop: HEADER_LEN + size as usize,
                leaf_hash_start: HEADER_LEN,
                leaf_hash_stop: HEADER_LEN,
            });
            let state = state.next_leaf();
            assert_eq!(state, LeafState::new_raw(size, 1));
            assert_eq!(state, LeafState {
                closed: true,
                object_size: size,
                leaf_index: 0,
                file_start: 0,
                file_stop: size,
                leaf_start: HEADER_LEN,
                leaf_stop: HEADER_LEN + size as usize,
                leaf_hash_start: HEADER_LEN,
                leaf_hash_stop: HEADER_LEN,
            });
        }

        let state = LeafState::new(1);
        assert_eq!(state.leaf_range(), 69..70);
        assert_eq!(state.commit_range(), 0..70);
        assert_eq!(state.is_small(), true);
        assert_eq!(state.is_large(), false);
        for size in [LEAF_SIZE + 1, 2 * LEAF_SIZE - 1, 2 * LEAF_SIZE] {
            let state = LeafState::new_raw(size, 0);
            assert_eq!(state, LeafState {
                closed: false,
                object_size: size,
                leaf_index: 0,
                file_start: 0,
                file_stop: LEAF_SIZE,
                leaf_start: HEADER_LEN + TUB_HASH_LEN * 2,
                leaf_stop: HEADER_LEN + TUB_HASH_LEN * 2 + LEAF_SIZE as usize,
                leaf_hash_start: HEADER_LEN,
                leaf_hash_stop: HEADER_LEN +  TUB_HASH_LEN,
            });
            let state = state.next_leaf();
            assert_eq!(state, LeafState::new_raw(size, 1));
            assert_eq!(state, LeafState {
                closed: false,
                object_size: size,
                leaf_index: 1,
                file_start: LEAF_SIZE,
                file_stop: size,
                leaf_start: HEADER_LEN + TUB_HASH_LEN * 2,
                leaf_stop: HEADER_LEN + TUB_HASH_LEN * 2 + (size - LEAF_SIZE) as usize,
                leaf_hash_start: HEADER_LEN + TUB_HASH_LEN,
                leaf_hash_stop: HEADER_LEN + TUB_HASH_LEN * 2,
            });
            let state = state.next_leaf();
            assert_eq!(state, LeafState::new_raw(size, 2));
            assert_eq!(state, LeafState {
                closed: true,
                object_size: size,
                leaf_index: 1,
                file_start: LEAF_SIZE,
                file_stop: size,
                leaf_start: HEADER_LEN + TUB_HASH_LEN * 2,
                leaf_stop: HEADER_LEN + TUB_HASH_LEN * 2 + (size - LEAF_SIZE) as usize,
                leaf_hash_start: HEADER_LEN + TUB_HASH_LEN,
                leaf_hash_stop: HEADER_LEN + TUB_HASH_LEN * 2,
            });
        }
    }

    #[test]
    fn test_leafstate_ranges() {
        for size in [1, 2, 3, LEAF_SIZE - 1, LEAF_SIZE] {
            let state = LeafState::new(size);
            assert!(state.is_small());
            assert_eq!(state.leaf_index, 0);
            assert_eq!(state.leaf_hashes_range(), 69..69);
            assert_eq!(state.leaf_range(), 69..69 + size as usize);
            assert_eq!(state.payload_range(), state.leaf_range());
            assert_eq!(state.commit_range(), 0..69 + size as usize);
        }

        for size in [LEAF_SIZE + 1, 2 * LEAF_SIZE - 1, 2 * LEAF_SIZE] {
            let state = LeafState::new(size);
            assert!(state.is_large());
            assert_eq!(state.payload_range(), state.leaf_hashes_range());
            assert_eq!(state.leaf_index, 0);
            assert_eq!(state.leaf_hashes_range(), 69..129);
            assert_eq!(state.leaf_range(), 129..129 + LEAF_SIZE as usize);
            assert_eq!(state.payload_range(), state.leaf_hashes_range());
            assert_eq!(state.commit_range(), 0..129);

            let state = state.next_leaf();
            assert!(state.is_large());
            assert_eq!(state.payload_range(), state.leaf_hashes_range());
            assert_eq!(state, LeafState::new_raw(size, 1));
            assert_eq!(state.leaf_index, 1);
            assert_eq!(state.leaf_hashes_range(), 69..129);
            assert_eq!(state.leaf_range(), 129..129 + (size - LEAF_SIZE) as usize);
            assert_eq!(state.commit_range(), 0..129);
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
        let head = TUB_HASH_LEN as u64 + 9;
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
        assert_eq!(get_full_object_size(1), (HEADER_LEN + 1) as u64);
        assert_eq!(get_full_object_size(2), (HEADER_LEN + 2) as u64);
        assert_eq!(get_full_object_size(3), (HEADER_LEN + 3) as u64);
        assert_eq!(get_full_object_size(LEAF_SIZE), HEADER_LEN as u64 + LEAF_SIZE);
        assert_eq!(get_full_object_size(LEAF_SIZE + 1),
            HEADER_LEN as u64 + TUB_HASH_LEN as u64 + LEAF_SIZE + 1
        );
    }

    #[test]
    fn test_get_buffer_size() {
        assert_eq!(get_buffer_size(1), (HEADER_LEN + 1) as u64);
        assert_eq!(get_buffer_size(2), (HEADER_LEN + 2) as u64);
        assert_eq!(get_buffer_size(3), (HEADER_LEN + 3) as u64);
        assert_eq!(get_buffer_size(LEAF_SIZE), HEADER_LEN as u64 + LEAF_SIZE);
        assert_eq!(get_buffer_size(LEAF_SIZE + 1),
            HEADER_LEN as u64 + TUB_HASH_LEN as u64 + LEAF_SIZE
        );
        assert_eq!(get_buffer_size(LEAF_SIZE + 1),
            HEADER_LEN as u64 + TUB_HASH_LEN as u64 + LEAF_SIZE
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
