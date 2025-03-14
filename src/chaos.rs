//! 💖 Content Hash Addressable Object Store (START HERE).
//!
//! An Object's wire format contains just four fields of the following sizes in
//! bytes:
//!
//! | Hash | Size | Kind | DATA       |
//! |------|------|------|------------|
//! |   30 |    3 |    1 | 1-16777216 |
//!
//! All objects start with a fixed size header (generic on `<N>`) followed by
//! 1 to 16777216 bytes of object data.  Empty objects (size=0) are not allowed.
//!
//! The size is "size minus one" encoded into 24 bits by first subtracting one
//! from the size.  In 24 bits you can store values from 0-16777215, but what
//! we actually want is 1-16777216.  So it works out just perfectly.
//!
//! Everything in Tub is framed within this object structure.  However, this
//! module is low level, does not handle things like large object encoding and
//! compression.  For that see `tub::inception`.
//!
//! This layer is smokin' fast. 🚀 Let's keep it that way!
//!
//! We have a strict budget for `Store.save()`, `Store.load()`, and
//! `Store.delete()`:
//!
//! 1.  A single system call to `write()` or `pread64()`
//! 2.  Zero heap allocations
//!
//! If we stick to the above, this should stay fast!
//!
//! We can get a bit more performance by replacing HashMap with something
//! custom... we already have a hash!  Maybe hash the Tub hash with aHash?

use getrandom;
use std::collections::HashMap;
use std::fs::File;
use std::io::Result as IoResult;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, SeekFrom};
use std::marker::PhantomData;
use std::os::unix::fs::FileExt;
use std::{cmp, fmt};

use crate::base::*;
use crate::dbase32::{db32dec_into, db32enc};
use crate::protocol::{Blake3, Hasher};

pub type DefaultName = Name<30>;
pub type DefaultObject = Object<Blake3, 30>;
pub type DefaultStore = Store<Blake3, 30>;

/// N byte long Tub name (content hash or random ID).
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Clone, Copy)]
pub struct Name<const N: usize> {
    pub buf: [u8; N],
}

impl<const N: usize> Name<N> {
    pub fn new() -> Self {
        Self { buf: [0_u8; N] }
    }

    pub fn from(src: &[u8]) -> Self {
        let buf: [u8; N] = src.try_into().expect("oops");
        Self { buf }
    }

    pub fn from_dbase32(txt: &str) -> Self {
        let mut buf = [0_u8; N];
        if db32dec_into(txt.as_bytes(), &mut buf) {
            Self { buf }
        } else {
            panic!("Handle this better, yo");
        }
    }

    pub fn randomize(&mut self) {
        getrandom::fill(&mut self.buf).unwrap();
    }

    pub fn into_buf(self) -> [u8; N] {
        self.buf
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn to_dbase32(&self) -> String {
        db32enc(&self.buf)
    }
}

impl<const N: usize> fmt::Display for Name<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_dbase32())
    }
}

impl<const N: usize> Default for Name<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Packs 24-bit `size` and 8-bit `kind` into a `u32`.
#[derive(Debug, PartialEq, Eq)]
pub struct Info {
    val: u32,
}

impl Info {
    fn new(size: usize, kind: u8) -> Self {
        if !(1..=OBJECT_MAX_SIZE).contains(&size) {
            panic!(
                "Info: Need 1 <= size <= {}; got size={}",
                OBJECT_MAX_SIZE, size
            );
        }
        Self {
            val: (size - 1) as u32 | (kind as u32) << 24,
        }
    }

    pub fn from_le_bytes(buf: &[u8]) -> Self {
        Self {
            val: u32::from_le_bytes(buf.try_into().expect("oops")),
        }
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

/*
Three cases:

.clear() resizes data to zero
write with obj.extend()
.finalize_with_kind() sets hash + size + kind

.reset() resizes data to size plus sets size + kind
read into obj.as_mut_data()
.finalize() sets hash


*/

/// Buffer containing a single object's header plus data.
#[derive(Debug, Default)]
pub struct Object<H: Hasher, const N: usize> {
    hasher: H,
    buf: Vec<u8>,
    cur: usize,
}

impl<H: Hasher, const N: usize> Object<H, N> {
    pub fn new() -> Self {
        Self {
            buf: vec![0; N + INFO_LEN],
            hasher: H::new(),
            cur: 0,
        }
    }

    pub fn into_buf(self) -> Vec<u8> {
        self.buf
    }

    pub fn reset(&mut self, size: usize, kind: u8) {
        self.buf.clear();
        self.buf.resize(N + INFO_LEN + size, 0);
        self.set_info(Info::new(size, kind));
    }

    pub fn clear(&mut self) {
        self.buf.resize(N + INFO_LEN, 0);
        self.buf.fill(0);
        self.cur = 0;
    }

    pub fn resize_to_info(&mut self) {
        self.buf.resize(N + INFO_LEN + self.info().size(), 0);
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn remaining(&self) -> usize {
        let max = N + INFO_LEN + OBJECT_MAX_SIZE;
        if self.len() > max {
            0
        } else {
            max - self.len()
        }
    }

    pub fn extend(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    // FIXME: We should not have this in the API, but super handy for testing and play
    pub fn randomize(&mut self, small: bool) -> Name<N> {
        getrandom::fill(&mut self.buf[N..N + INFO_LEN]).unwrap();
        if small {
            self.buf[N] = cmp::max(self.buf[N], 15);
            self.buf[N + 1] = 0;
            self.buf[N + 2] = 0;
        }
        self.resize_to_info();
        getrandom::fill(self.as_mut_data()).unwrap();
        self.finalize_with_kind(ObjKind::Data as u8)
    }

    pub fn fast_randomize(&mut self) -> Name<N> {
        let mut h = blake3::Hasher::new();
        h.update(self.as_data());
        h.finalize_xof().fill(self.as_mut_data());
        self.finalize()
    }

    pub fn compute(&self) -> Name<N> {
        let mut hash: Name<N> = Name::new();
        self.hasher.hash_into(self.as_payload(), hash.as_mut_buf());
        hash
    }

    pub fn is_valid(&self) -> bool {
        self.hash() == self.compute()
    }

    pub fn validate_against(&self, hash: &Name<N>) -> bool {
        self.is_valid() && hash == &self.hash()
    }

    pub fn finalize(&mut self) -> Name<N> {
        assert_eq!(self.info().size(), self.as_data().len());
        assert_eq!(self.buf.len(), N + INFO_LEN + self.info().size());
        let hash = self.compute();
        self.buf[0..N].copy_from_slice(hash.as_buf());
        hash
    }

    pub fn finalize_with_kind(&mut self, kind: u8) -> Name<N> {
        self.set_info(Info::new(self.as_data().len(), kind));
        self.finalize()
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

    pub fn kind(&self) -> ObjKind {
        self.buf[N + 3].into()
    }

    pub fn set_kind(&mut self, kind: ObjKind) {
        self.buf[N + 3] = kind as u8;
    }

    pub fn raw_kind(&self) -> u8 {
        self.buf[N + 3]
    }

    pub fn set_raw_kind(&mut self, raw_kind: u8) {
        self.buf[N + 3] = raw_kind;
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

    pub fn as_header(&self) -> &[u8] {
        &self.buf[0..N + INFO_LEN]
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

    pub fn as_payload(&self) -> &[u8] {
        &self.buf[N..]
    }
}

impl<H: Hasher, const N: usize> fmt::Display for Object<H, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {:?}",
            self.hash(),
            self.info().size(),
            self.kind()
        )
    }
}

/// A value in the `Store.map` HashMap index.
pub struct Entry {
    pub info: Info,
    pub offset: u64,
}

impl Entry {
    pub fn new(info: Info, offset: u64) -> Self {
        Self { info, offset }
    }
}

// Read objects from an object stream.
pub struct ObjectReader<'a, R: Read, H: Hasher, const N: usize> {
    phantom1: PhantomData<R>, // This feels like me babysitting the compiler 🤪
    phantom2: PhantomData<H>,
    inner: &'a mut R,
}

impl<'a, R: Read, H: Hasher, const N: usize> ObjectReader<'a, R, H, N> {
    pub fn new(reader: &'a mut R) -> Self {
        Self {
            phantom1: PhantomData,
            phantom2: PhantomData,
            inner: reader,
        }
    }

    pub fn read_next(&mut self, obj: &mut Object<H, N>) -> IoResult<bool> {
        obj.clear();
        if self.inner.read_exact(obj.as_mut_header()).is_ok() {
            obj.resize_to_info();
            self.inner.read_exact(obj.as_mut_data())?;
            if !obj.is_valid() {
                panic!("Not valid {}", obj.hash()); // FIXME: handle more better
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Organizes objects in an append-only file.
pub struct Store<H: Hasher, const N: usize> {
    file: File,
    _hasher: H,
    map: HashMap<Name<N>, Entry>,
    offset: u64,
}

impl<H: Hasher, const N: usize> Store<H, N> {
    pub fn new(file: File) -> Self {
        Self {
            file,
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

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn size(&self) -> u64 {
        self.offset
    }

    pub fn keys(&self) -> Vec<Name<N>> {
        Vec::from_iter(self.map.keys().cloned())
    }

    pub fn reindex(&mut self, obj: &mut Object<H, N>) -> IoResult<()> {
        self.map.clear();
        self.offset = 0;
        self.file.rewind()?;
        let mut br = BufReader::new(self.file.try_clone()?);
        let mut reader: ObjectReader<BufReader<File>, H, N> = ObjectReader::new(&mut br);
        while reader.read_next(obj)? {
            self.map
                .insert(obj.hash(), Entry::new(obj.info(), self.offset));
            self.offset += obj.len() as u64;
        }
        // Truncate to end of valid object stream, discarding any partial object
        self.file.rewind()?; // Needed on Windows
        self.file.set_len(self.offset)?;
        obj.clear();
        Ok(())
    }

    pub fn reindex_from(&mut self, obj: &mut Object<H, N>, idx: File) -> IoResult<()> {
        self.map.clear();
        self.offset = 0;

        // Load entries from the saved index file
        let mut idx = BufReader::new(idx);
        while idx.read_exact(obj.as_mut_header()).is_ok() {
            self.map
                .insert(obj.hash(), Entry::new(obj.info(), self.offset));
            self.offset += (N + 4 + obj.info().size()) as u64;
        }
        // FIXME: truncate if needed based on OFFSET % HEADER_LEN

        // Index plus verify remaining objects, adding to index file
        let mut idx = BufWriter::new(idx.into_inner());
        self.file.seek(SeekFrom::Start(self.offset))?; // Very important!
        let mut br = BufReader::new(self.file.try_clone()?);
        let mut reader: ObjectReader<BufReader<File>, H, N> = ObjectReader::new(&mut br);
        while reader.read_next(obj)? {
            self.map
                .insert(obj.hash(), Entry::new(obj.info(), self.offset));
            idx.write_all(obj.as_header())?;
            self.offset += (N + 4 + obj.info().size()) as u64;
        }
        // Truncate to end of valid object stream, discarding any partial object
        self.file.seek(SeekFrom::Start(self.offset))?; // Needed on Windows
        self.file.set_len(self.offset)?;
        idx.flush()?;
        obj.clear();
        Ok(())
    }

    pub fn load_unchecked(&mut self, hash: &Name<N>, obj: &mut Object<H, N>) -> IoResult<bool> {
        if let Some(entry) = self.map.get(hash) {
            obj.reset(entry.info.size(), entry.info.kind());
            self.file.read_exact_at(obj.as_mut_buf(), entry.offset)?;
            /* This is the slow path without pread64():
            self.file.seek(SeekFrom::Start(entry.offset))?;
            self.file.read_exact(obj.as_mut_buf())?;
            */
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn load(&mut self, hash: &Name<N>, obj: &mut Object<H, N>) -> IoResult<bool> {
        if self.load_unchecked(hash, obj)? {
            if !obj.validate_against(hash) {
                panic!("{} hash does not match", hash);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn save(&mut self, obj: &Object<H, N>) -> IoResult<bool> {
        let hash = obj.hash();
        let info = obj.info();
        if let Some(_entry) = self.map.get(&hash) {
            Ok(false)
        } else {
            self.file.write_all(obj.as_buf())?;
            self.map.insert(hash, Entry::new(info, self.offset));
            self.offset += obj.len() as u64;
            Ok(true)
        }
    }

    pub fn delete(&mut self, _hash: Name<N>) -> IoResult<bool> {
        // FIXME: Decide how tombstones should work with new new
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::{TestTempDir, flip_bit_in};
    use std::collections::HashSet;

    #[test]
    fn test_name() {
        let mut n = Name::<30>::new();
        assert_eq!(n.len(), 30);
        assert_eq!(n.as_buf(), [0_u8; 30]);
        assert_eq!(n.as_mut_buf(), [0_u8; 30]);
        assert_eq!(
            n.to_string(),
            "333333333333333333333333333333333333333333333333"
        );
        n.as_mut_buf().fill(255);
        assert_eq!(n.len(), 30);
        assert_eq!(n.as_buf(), [255_u8; 30]);
        assert_eq!(n.as_mut_buf(), [255_u8; 30]);
        assert_eq!(
            n.to_string(),
            "YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY"
        );

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
    fn test_name_randomize() {
        let mut set: HashSet<DefaultName> = HashSet::new();
        let mut name = DefaultName::new();
        for _ in 0..777 {
            name.randomize();
            set.insert(name.clone());
        }
        assert_eq!(set.len(), 777);
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
    #[should_panic(expected = "Need 1 <= size <= 16777216; got size=0")]
    fn test_info_panic_low() {
        let _sk = Info::new(0, 0);
    }

    #[test]
    #[should_panic(expected = "Need 1 <= size <= 16777216; got size=16777217")]
    fn test_info_panic_high() {
        let _sk = Info::new(OBJECT_MAX_SIZE + 1, 0);
    }

    #[test]
    fn test_object() {
        let mut obj: Object<Blake3, 30> = Object::new();
        assert_eq!(obj.len(), 34);
        obj.clear();
        assert_eq!(obj.len(), 34);

        assert_eq!(obj.info().size(), 1);
        assert_eq!(obj.info().kind(), 0);
        assert_eq!(obj.raw_kind(), 0);
        assert_eq!(obj.as_buf(), &[0; 34]);

        obj.as_mut_buf().fill(255);
        assert_eq!(obj.info().size(), 16 * 1024 * 1024);
        assert_eq!(obj.info().kind(), 255);
        assert_eq!(obj.raw_kind(), 255);

        assert_eq!(obj.len(), 34);
        assert_eq!(obj.as_buf(), &[255; 34]);

        obj.clear();
        assert_eq!(obj.len(), 34);
        assert_eq!(obj.as_buf(), &[0; 34]);
        assert_eq!(obj.as_payload(), &[0; 4]);

        for k in 0_u8..=255 {
            obj.set_raw_kind(k);
            assert_eq!(obj.raw_kind(), k);
            assert_eq!(obj.info().kind(), k);
        }
    }

    #[test]
    fn test_object_validity() {
        let mut obj: Object<Blake3, 30> = Object::new();
        obj.randomize(true);
        assert!(obj.is_valid());
        for bit in 0..obj.len() * 8 {
            flip_bit_in(obj.as_mut_buf(), bit);
            assert!(!obj.is_valid());
            flip_bit_in(obj.as_mut_buf(), bit);
            assert!(obj.is_valid());
        }

        let mut hash = obj.hash();
        for bit in 0..hash.len() * 8 {
            flip_bit_in(hash.as_mut_buf(), bit);
            assert!(!obj.validate_against(&hash));
            flip_bit_in(hash.as_mut_buf(), bit);
            assert!(obj.validate_against(&hash));
        }
    }

    #[test]
    fn test_store() {
        let tmp = TestTempDir::new();
        let path = tmp.build(&["foo"]);
        let file = File::options()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)
            .unwrap();
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
            obj2.clear();
            assert!(store.load(&hash1, &mut obj2).unwrap());
            assert_eq!(obj1.as_buf(), obj2.as_buf());
        }
        for _ in 0..256 {
            obj1.randomize(true);
            let hash1 = obj1.hash();
            assert!(store.save(&obj1).unwrap());
            assert!(store.map.contains_key(&hash1));
            obj2.clear();
            assert!(store.load(&hash1, &mut obj2).unwrap());
            assert_eq!(obj1.as_buf(), obj2.as_buf());
        }

        let keys = store.keys();
        for key in keys.iter() {
            assert!(store.load(&key, &mut obj1).unwrap());
        }
        store.reindex(&mut obj1).unwrap();
        assert_eq!(store.len(), keys.len());
        for key in keys.iter() {
            assert!(store.load(&key, &mut obj1).unwrap());
        }
    }
}
