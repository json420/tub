//! Low-level object store.

/* FIXME:

Ideally we should do filesystem IO relative to an open directory descriptor.
However, this is not currently supported in the Rust standard library.

There are crates like `openat` and `openat_ct`, but they aren't under very
active development and likely lack features we need.

So to get to MVP as quickly as possible, we'll do normal absolute path IO, then
switch later.

But to be clear: doing IO relative to an open directory descriptor is definitely
the most modern and correct way to do this sort of thing.  So let's do that!
*/

use std::path::{Path, PathBuf};
use std::io::prelude::*;
use std::io;
use std::os::unix::fs::FileExt;
use std::fs;
use std::fs::File;
use std::collections::HashMap;

use tempfile::TempDir;

use crate::base::*;
use crate::protocol::{hash_tombstone};
use crate::dbase32::{db32enc_str, Name2Iter};
use crate::util::random_id;
use crate::leaf_io::{Object, get_preamble_size};
use crate::leaf_io::{TubBuf, LeafReader, TmpObject, ReindexBuf};


macro_rules! other_err {
    ($msg:literal) => {
        Err(io::Error::new(io::ErrorKind::Other, $msg))
    }
}


/// An entry in the HashMap index.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Entry {
    kind: ObjectType,
    size: u64,
    offset: u64,
}

impl Entry {
    pub fn new(size: u64, offset: u64) -> Self {
        Self {kind: ObjectType::Data, size: size, offset: offset}
    }

    pub fn data_offset(&self) -> u64 {
        self.offset + get_preamble_size(self.size)
    }

    pub fn is_large(&self) -> bool {
        self.size > LEAF_SIZE
    }

    pub fn is_small(&self) -> bool {
        ! self.is_large()
    }
}


type Index = HashMap<TubHash, Entry>;


pub fn find_store(path: &Path) -> io::Result<Store>
{
    let mut pb = PathBuf::from(path);
    loop {
        pb.push(DOTDIR);
        if pb.is_dir() {
            return Store::new(&pb);
        }
        pb.pop();
        if !pb.pop() {
            return other_err!("cannot find control directory");
        }
    }
}


/// Initialize a store layout in an empty directory.
pub fn init_store(path: &Path) -> io::Result<Store>
{
    let mut pb = PathBuf::from(path);

    // objects directory and sub-directories
    pb.push(OBJECTDIR);
    fs::create_dir(pb.as_path())?;
    for name in Name2Iter::new() {
        pb.push(name);
        fs::create_dir(pb.as_path())?;
        pb.pop();
    }
    pb.pop();

    // partial directory:
    pb.push(PARTIALDIR);
    fs::create_dir(pb.as_path())?;
    pb.pop();

    // tmp directory:
    pb.push(TMPDIR);
    fs::create_dir(pb.as_path())?;
    pb.pop();

    // REAMDE file  :-)
    pb.push(README);
    let mut f = fs::File::create(pb.as_path())?;
    f.write_all(README_CONTENTS)?;
    pb.pop();

    Store::new(path)
}


/// Creates the `".bathtub_db"` directory, calls `init_store()`.
pub fn init_tree(path: &Path) -> io::Result<Store>
{
    let mut pb = PathBuf::from(path);
    pb.push(DOTDIR);
    fs::create_dir(&pb)?;
    init_store(&pb)
}

fn push_pack_path(pb: &mut PathBuf) {
    pb.push(PACKFILE);
}

fn push_old_pack_path(pb: &mut PathBuf) {
    pb.push(PACKFILE);
    pb.set_extension("db.old");
}

fn push_object_path(pb: &mut PathBuf, id: &TubHash) {
    pb.push(OBJECTDIR);
    let sid = db32enc_str(id);
    let (prefix, suffix) = sid.split_at(2);
    pb.push(prefix);
    pb.push(suffix);
}

fn push_partial_path(pb: &mut PathBuf, id: &TubHash) {
    pb.push(PARTIALDIR);
    pb.push(db32enc_str(id));
}

fn push_tmp_path(pb: &mut PathBuf, key: &TubId) {
    pb.push(TMPDIR);
    pb.push(db32enc_str(key));
}


pub struct Summary {
    count: u64,
    total: u64,
}
impl Summary {
    pub fn new() -> Self {
        Self {count: 0, total: 0}
    }

    pub fn increment(&mut self, entry: &Entry) {
        self.count += 1;
        self.total += entry.size;
    }
}

pub struct Stats {
    large: Summary,
    small: Summary,
    data: Summary,
    tree: Summary,
}
impl Stats {
    pub fn new() -> Self {
        Self {
            large: Summary::new(),
            small: Summary::new(),
            data: Summary::new(),
            tree: Summary::new(),
        }
    }

    pub fn increment(&mut self, entry: &Entry) {
        if entry.is_large() {
            self.large.increment(entry);
        }
        else {
            self.small.increment(entry);
        }
        match entry.kind {
            ObjectType::Data => {
                self.data.increment(entry);
            },
            ObjectType::Tree => {
                self.tree.increment(entry);
            },
        }
    }
}


/// Layout of large and small objects on the filesystem.
#[derive(Debug)]
pub struct Store {
    tbuf: TubBuf,  // FIXME this wont work for multi-threaded
    path: PathBuf,
    file: fs::File,
    index: Index,
    offset: u64,
}

// FIXME: for multithread, Store needs to be wrapped in Arc<Mutex<>>
impl Store {
    pub fn new(path: &Path) -> io::Result<Self>
    {
        let tbuf = TubBuf::new();
        let pb = PathBuf::from(path);

        let mut pb_copy = pb.clone();
        push_pack_path(&mut pb_copy);
        let file = File::options()
                        .read(true)
                        .append(true)
                        .create(true).open(pb_copy)?;
        Ok(
            Store {tbuf: tbuf, path: pb, file: file, index: HashMap::new(), offset: 0}
        )
    }

    // FIXME: This is mostly for testing and play, but perhaps should be
    // removed after MVP.
    pub fn new_tmp() -> (TempDir, Self) {
        let tmp = TempDir::new().unwrap();
        //let store = Store::new(tmp.path()).unwrap();
        let store = init_store(tmp.path()).unwrap();
        (tmp, store)
    }

    /// Returns clone of self.path
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn pack_path(&self) -> PathBuf {
        let mut pb = self.path();
        push_pack_path(&mut pb);
        pb
    }

    pub fn old_pack_path(&self) -> PathBuf {
        let mut pb = self.path();
        push_old_pack_path(&mut pb);
        pb
    }

    /// Builds canonical large file path.
    pub fn object_path(&self, id: &TubHash) -> PathBuf {
        let mut pb = self.path();
        push_object_path(&mut pb, id);
        pb
    }

    /// Builds canonical partial large file path.
    ///
    /// A "partial" object is an object whose hash/ID is known, but not all the
    /// its leaves are present in this store.
    pub fn partial_path(&self, id: &TubHash) -> PathBuf {
        let mut pb = self.path();
        push_partial_path(&mut pb, id);
        pb
    }

    pub fn tmp_path(&self, id: &TubId) -> PathBuf {
        let mut pb = self.path();
        push_tmp_path(&mut pb, id);
        pb
    }

    pub fn open_tmp(&self, id: &TubId) -> io::Result<(PathBuf, File)>  {
        let pb = self.tmp_path(id);
        let file = File::options().append(true).create_new(true).open(&pb)?;
        Ok((pb, file))
    }

    pub fn allocate_tmp(&self) -> io::Result<TmpObject>
    {
        let id = random_id();
        let path = self.tmp_path(&id);
        TmpObject::new(id, path)
    }

    pub fn finalize_tmp(&mut self, mut tmp: TmpObject, hash: &TubHash) -> io::Result<()>
    {
        let from = tmp.pb;
        let to = self.object_path(hash);
        fs::rename(&from, &to)
    }

    fn open_large(&self, id: &TubHash) -> io::Result<fs::File> {
        File::open(self.object_path(id))
    }

    fn remove_large(&self, id: &TubHash) -> io::Result<()> {
        let pb = self.object_path(id);
        eprintln!("Deleting {:?}", pb);
        fs::remove_file(pb)
    }

    pub fn open(&self, hash: &TubHash) -> io::Result<Option<Object>> {
        if let Some(entry) = self.index.get(hash) {
            let obj = match entry.is_large() {
                true => {
                    let file = self.open_large(&hash)?;
                    Object::new(file, entry.size, 0)
                }
                false => {
                    let file = self.file.try_clone()?;
                    Object::new(file, entry.size, entry.data_offset())
                }
            };
            Ok(Some(obj))
        }
        else {
            Ok(None)
        }
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn keys(&self) -> Vec<TubHash> {
        Vec::from_iter(self.index.keys().cloned())
    }

    pub fn sync_data(&mut self) {
        self.file.flush().expect("nope");
        self.file.sync_data().expect("nope");
    }

    pub fn reindex(&mut self) -> io::Result<()>
    {
        self.index.clear();
        self.offset = 0;
        let mut tombstones = 0_u64;
        let mut rbuf = ReindexBuf::new();
        while let Ok(_) = self.file.read_exact_at(rbuf.as_mut_buf(), self.offset) {
            if rbuf.is_object() {
                let entry = Entry::new(rbuf.size(), self.offset);
                self.index.insert(rbuf.hash(), entry);
            }
            else if rbuf.is_tombstone() {
                tombstones += 1;
                println!("Tombstone: {}", rbuf);
                if self.index.remove(&rbuf.hash()) == None {
                    panic!("{} not in index but tombstone found", self.offset);
                }
            }
            else {
                panic!("bad entry: {}", rbuf);
            }
            assert_eq!(rbuf.object_type(), ObjectType::Data);
            self.offset += rbuf.offset_size();
            rbuf.reset();
        }
        if tombstones > 0 {
            eprintln!("Found {} tombstones", tombstones);
        }
        // Was there any leftover?
        let leftover = self.file.read_at(rbuf.as_mut_buf(), self.offset)?;
        if leftover > 0 {
            // FIXME: should we write dangling bits to a backup file?
            eprintln!("Trunkcating to {} bytes", self.offset);
            self.file.set_len(self.offset)?;
        }
        eprintln!("Indexed {} objects", self.len());
        Ok(())
    }

    pub fn repack(&mut self) -> io::Result<()> {
        // FIXME: Currently we do this in arbitrary order (what HashMap.iter()
        // gives us), but we'll obviously get better performance if we go
        // through the file sequentially.  Note that semantically the order
        // doesn't matter, it's just a performance issue.
        //
        // The only time the "order" of the pack file matters is with
        // tombstones.  A tombstone after the corresponding object means that
        // object is deleted, whereas a tombstone before the object is invalid.
        // Note that tombstones are not copied into the new pack file (which is
        // why the order doesn't matter).
        //
        // We should probably walk through the file again like Store.reindex()
        // does, it just adds some complexity.
        let id = random_id();
        let (tmp_pb, mut tmp) = self.open_tmp(&id)?;
        for (_hash, entry) in self.index.iter() {
            assert!(entry.size > 0);
            self.tbuf.resize(entry.size);
            self.file.read_exact_at(self.tbuf.as_mut_commit(), entry.offset)?;
            if self.tbuf.is_valid_for_commit() {
                tmp.write_all(self.tbuf.as_commit())?;
            }
            else {
                panic!("shit is broke, yo");
            }
        }
        tmp.sync_all()?;
        fs::rename(self.pack_path(), self.old_pack_path())?;
        fs::rename(&tmp_pb, self.pack_path())?;
        self.file = File::options().read(true).append(true).open(self.pack_path())?;
        self.reindex()?;
        Ok(())
    }

    pub fn stats(&self) -> Stats {
        let mut stats = Stats::new();
        for entry in self.index.values() {
            stats.increment(entry);
        }
        stats
    }

    pub fn import_file(&mut self, mut file: File, size: u64) -> io::Result<(TubHash, bool)> {
        self.tbuf.resize(size);
        if self.tbuf.is_small() {
            file.read_exact(self.tbuf.as_mut_leaf().unwrap())?;
            self.tbuf.finalize();
        }
        else {
            let mut tmp = self.allocate_tmp()?;
            while let Some(buf) = self.tbuf.as_mut_leaf() {
                file.read_exact(buf)?;
                tmp.write_all(self.tbuf.as_leaf())?;
                self.tbuf.hash_leaf();
            }
            assert_eq!(tmp.total, size);
            self.tbuf.finalize();
            self.finalize_tmp(tmp, &self.tbuf.hash())?;
        }
        self.commit_object()
    }

    pub fn commit_object(&mut self) -> io::Result<(TubHash, bool)>
    {
        let hash = self.tbuf.hash();
        if let Some(_entry) = self.index.get(&hash) {
            Ok((hash, false))  // Already in object store
        }
        else {
            let entry = Entry::new(self.tbuf.size(), self.offset);
            self.index.insert(hash, entry);
            self.file.write_all(self.tbuf.as_commit())?;
            self.offset += self.tbuf.as_commit().len() as u64;
            Ok((hash, true))
        }
    }

    pub fn add_object(&mut self, data: &[u8]) -> io::Result<(TubHash, bool)> {
        self.tbuf.hash_data(data);
        self.commit_object()
    }

    pub fn get_object(&mut self, id: &TubHash, _verify: bool) -> io::Result<Option<Vec<u8>>>
    {
        if let Some(entry) = self.index.get(id) {
            let mut buf = vec![0_u8; entry.size as usize];
            self.file.read_exact_at(&mut buf, entry.data_offset())?;
            /*  FIXME
            if verify && id != &hash(&buf).hash {
                eprintln!("{} is corrupt", db32enc_str(id));
                self.delete_object(id)?;
            }
            */
            Ok(Some(buf))
        }
        else {
            Ok(None)
        }
    }

    pub fn get_object_new(&mut self, id: &TubHash, buf: &mut Vec<u8>) -> io::Result<bool>
    {
        if let Some(entry) = self.index.get(id) {
            buf.resize(entry.size as usize, 0);
            assert_eq!(buf.len() as u64, entry.size);
            self.file.read_exact_at(buf, entry.data_offset())?;
            Ok(true)
        }
        else {
            Ok(false)
        }
    }

    pub fn delete_object(&mut self, hash: &TubHash) -> io::Result<bool> {
        /*  Remove an object from the Store.

        This writes a tombstone to the pack file and then, in the large object
        case, remove the corresponding o/AA/AAA... file.  When the next repack
        occurs, the object entry in the pack file and the tombstone will be
        removed (not copied into the new pack file).
        */
        if let Some(entry) = self.index.get(hash) {
            eprintln!("Deleting {}", db32enc_str(hash));
            let mut buf = [0_u8; HEADER_LEN];
            buf[ROOT_HASH_RANGE].copy_from_slice(hash);
            buf[PAYLOAD_HASH_RANGE].copy_from_slice(&hash_tombstone(hash));
            self.file.write_all(&buf)?;
            self.offset += buf.len() as u64;
            if entry.is_large() {
                self.remove_large(hash)?;
            }
            self.index.remove(hash);
            Ok(true)
        }
        else {
            Ok(false)  // object not in this store
        }
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbase32::Name2Iter;
    use crate::helpers::TestTempDir;
    use crate::util::{random_hash, random_small_object};

    #[test]
    fn test_entry() {
        let entry = Entry::new(1, 7);
        assert_eq!(entry.size, 1);
        assert_eq!(entry.offset, 7);
        assert_eq!(entry.data_offset(), 7 + get_preamble_size(1));
        assert_eq!(entry.is_large(), false);
        assert_eq!(entry.is_small(), true);

        let entry = Entry::new(LEAF_SIZE + 1, 11);
        assert_eq!(entry.size, LEAF_SIZE + 1);
        assert_eq!(entry.offset, 11);
        assert_eq!(entry.data_offset(), 11 + get_preamble_size(LEAF_SIZE + 1));
        assert_eq!(entry.is_large(), true);
        assert_eq!(entry.is_small(), false);
    }

    #[test]
    fn test_find_store() {
        let tmp = TestTempDir::new();

        // We're gonna use these over and over:
        let tree = tmp.pathbuf();
        let dotdir = tmp.build(&[DOTDIR]);
        let foo = tmp.build(&["foo"]);
        let bar = tmp.build(&["foo", "bar"]);
        let empty: Vec<String> = vec![];

        // tmp.path() is an empty directory still:
        assert!(find_store(&tree).is_err());
        assert!(find_store(&dotdir).is_err());
        assert!(find_store(&foo).is_err());
        assert!(find_store(&bar).is_err());

        // Nothing should have been created
        assert_eq!(tmp.list_root(), empty);

        // create foo/bar, but still no DOTDIR
        assert_eq!(tmp.makedirs(&["foo", "bar"]), bar);

        assert!(find_store(&tree).is_err());
        assert!(find_store(&dotdir).is_err());
        assert!(find_store(&foo).is_err());
        assert!(find_store(&bar).is_err());

        // Still nothing should have been created by find_store():
        assert_eq!(tmp.list_root(), ["foo"]);
        assert_eq!(tmp.list_dir(&["foo"]), ["bar"]);
        assert_eq!(tmp.list_dir(&["foo", "bar"]), empty);

    }

    #[test]
    fn test_push_pack_path() {
        let mut pb = PathBuf::new();
        push_pack_path(&mut pb);
        assert_eq!(pb.as_os_str(), "bathtub.db");
    }

    #[test]
    fn test_push_object_path() {
        let id = [0_u8; TUB_HASH_LEN];
        let mut pb = PathBuf::new();
        push_object_path(&mut pb, &id);
        assert_eq!(pb.as_os_str(),
            "objects/33/3333333333333333333333333333333333333333333333"
        );
    }

    #[test]
    fn test_push_partial_path() {
        let id = [0_u8; TUB_HASH_LEN];
        let mut pb = PathBuf::new();
        push_partial_path(&mut pb, &id);
        assert_eq!(pb.as_os_str(),
            "partial/333333333333333333333333333333333333333333333333"
        );
    }

    #[test]
    fn test_push_tmp_path() {
        let key = [0_u8; TUB_ID_LEN];
        let mut pb = PathBuf::new();
        push_tmp_path(&mut pb, &key);
        assert_eq!(pb.as_os_str(),
            "tmp/333333333333333333333333"
        );
    }

    #[test]
    fn test_init_tree() {
        let tmp = TestTempDir::new();

        let _store = init_tree(tmp.path()).unwrap();
        assert_eq!(tmp.list_root(), vec![DOTDIR]);

        let mut expected = vec![OBJECTDIR, PARTIALDIR, TMPDIR, README, PACKFILE];
        expected.sort();
        assert_eq!(tmp.list_dir(&[DOTDIR]), expected);

        let dirs = tmp.list_dir(&[DOTDIR, OBJECTDIR]);
        assert_eq!(dirs.len(), 1024);
        let expected: Vec<String> = Name2Iter::new().collect();
        assert_eq!(dirs, expected);
        assert_eq!(dirs[0], "33");
        assert_eq!(dirs[1], "34");
        assert_eq!(dirs[1022], "YX");
        assert_eq!(dirs[1023], "YY");
    }

    #[test]
    fn test_init_store() {
        let tmp = TestTempDir::new();
        let mut pb = PathBuf::from(tmp.pathbuf());
        init_store(&mut pb).unwrap();
        let mut expected = vec![OBJECTDIR, PARTIALDIR, TMPDIR, README, PACKFILE];
        expected.sort();
        assert_eq!(tmp.list_root(), expected);
        let dirs = tmp.list_dir(&[OBJECTDIR]);
        assert_eq!(dirs.len(), 1024);
        let expected: Vec<String> = Name2Iter::new().collect();
        assert_eq!(dirs, expected);
        assert_eq!(dirs[0], "33");
        assert_eq!(dirs[1], "34");
        assert_eq!(dirs[1022], "YX");
        assert_eq!(dirs[1023], "YY");
    }

    #[test]
    fn test_store_delete_object() {
        let (_tmp, mut store) = Store::new_tmp();
        assert_eq!(store.offset, 0);
        let hash = random_hash();
        assert!(! store.delete_object(&hash).unwrap());
        assert_eq!(store.offset, 0);
        let obj = random_small_object();
        let (hash2, new) = store.add_object(&obj).unwrap();
        assert_eq!(store.len(), 1);
        assert_ne!(hash, hash2);
        assert!(new);
        assert_eq!(store.offset, (HEADER_LEN + obj.len()) as u64);

        // Trying to delete non-existing object should still do nothing
        assert!(! store.delete_object(&hash).unwrap());
        assert_eq!(store.offset, (HEADER_LEN + obj.len()) as u64);

        // Delete obj
        assert!(store.delete_object(&hash2).unwrap());
        assert_eq!(store.offset, (2 * HEADER_LEN + obj.len()) as u64);
        assert_eq!(store.len(), 0);

        // Try to delete both again, should do nothing
        assert!(! store.delete_object(&hash).unwrap());
        assert!(! store.delete_object(&hash2).unwrap());
        assert_eq!(store.offset, (2 * HEADER_LEN + obj.len()) as u64);
        assert_eq!(store.len(), 0);
    }
}

