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
use crate::protocol::{hash, hash_tombstone};
use crate::dbase32::{db32enc_str, Name2Iter};
use crate::util::random_id;
use crate::leaf_io::{Object, LeafReader, new_leaf_buf, TubTop, TmpObject, data_offset};


macro_rules! other_err {
    ($msg:literal) => {
        Err(io::Error::new(io::ErrorKind::Other, $msg))
    }
}


/// An entry in the HashMap index.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Entry {
    size: u64,
    offset: u64,
}

impl Entry {
    pub fn new(size: u64, offset: u64) -> Self {
        Self {size: size, offset: offset}
    }

    pub fn data_offset(&self) -> u64 {
        self.offset + data_offset(self.size)
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

fn push_repack_path(pb: &mut PathBuf, id: &TubId) {
    pb.push(PACKFILE);
    let sid = db32enc_str(id);
    pb.set_extension(sid);
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


pub enum NewObj<'a> {
    File(TmpObject),
    Mem(&'a [u8]),
}


/// Layout of large and small objects on the filesystem.
#[derive(Debug)]
pub struct Store {
    path: PathBuf,
    file: fs::File,
    index: Index,
    offset: u64,
}

// FIXME: for multithread, Store needs to be wrapped in Arc<Mutex<>>
impl Store {
    pub fn new(path: &Path) -> io::Result<Self>
    {
        let pb = PathBuf::from(path);

        let mut pb_copy = pb.clone();
        pb_copy.push(PACKFILE);
        let file = File::options()
                        .read(true)
                        .append(true)
                        .create(true).open(pb_copy)?;
        Ok(
            Store {path: pb, file: file, index: HashMap::new(), offset: 0}
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

    pub fn repack_path(&self, id: &TubId) -> PathBuf {
        let mut pb = self.path();
        push_repack_path(&mut pb, id);
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

    pub fn allocate_tmp(&self) -> io::Result<TmpObject>
    {
        let id = random_id();
        let path = self.tmp_path(&id);
        TmpObject::new(id, path)
    }

    pub fn finalize_tmp(&mut self, tmp: TmpObject, hash: &TubHash) -> io::Result<()>
    {
        let from = tmp.path;
        let to = self.object_path(hash);
        let mut to_parent = to.clone();
        to_parent.pop();
        fs::create_dir_all(&to_parent)?;
        fs::rename(&from, &to)
    }

    pub fn import_file(&mut self, file: File) -> io::Result<(TubTop, bool)>
    {
        let mut reader = LeafReader::new(file);
        let mut tmp = self.allocate_tmp()?;
        let mut buf = new_leaf_buf();
        while let Some(_info) = reader.read_next_leaf(&mut buf)? {
            tmp.write_leaf(&buf)?;
        }
        let tt = reader.finalize();
        let new = self.commit_object(&tt, NewObj::File(tmp))?;
        Ok((tt, new))
    }

    fn open_large(&self, id: &TubHash) -> io::Result<fs::File> {
        File::open(self.object_path(id))
    }

    fn remove_large(&self, id: &TubHash) -> io::Result<()> {
        eprintln!("Deleting {}", db32enc_str(id));
        fs::remove_file(self.object_path(id))
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
        let mut tt = TubTop::new();
        while let Ok(_) = self.file.read_exact_at(tt.as_mut_head(), self.offset) {
            let hash = tt.hash();
            let size = tt.size();
            if size == 0 {
                // Deletion tombstone
                if ! tt.is_tombstone() {
                    panic!("bad tombstone {}; offset={}", tt, self.offset);   
                }
                if self.index.remove(&hash) == None {
                    panic!("{} not in index but tombstone found", tt);
                }
            }
            else {
                if tt.is_large() {
                    // More than one leaf, read in remaining leaf hashes
                    tt.resize_to_size();
                    self.file.read_exact_at(
                        tt.as_mut_tail(), self.offset + HEAD_LEN as u64
                    )?;
                }
                if ! tt.is_valid() {
                    panic!("not valid: {}; offset={}", tt, self.offset);
                }
                let entry = Entry::new(size, self.offset);
                self.index.insert(hash, entry);
                if tt.is_small() {
                    // Only small objects are in self.file
                    self.offset += size;
                }
            }
            self.offset += tt.len() as u64;
            tt.reset();
        }
        // Was there any leftover?
        let leftover = self.file.read_at(tt.as_mut_head(), self.offset)?;
        if leftover > 0 {
            // FIXME: should we write dangling bits to a backup file?
            eprintln!("Trunkcating to {} bytes", self.offset);
            self.file.set_len(self.offset)?;
        }
        Ok(())
    }

    pub fn repack(&mut self) -> io::Result<()> {
        let id = random_id();
        let tmp_pb = self.repack_path(&id);
        let mut tmp = File::options().append(true).create_new(true).open(&tmp_pb)?;
        let mut tt = TubTop::new();
        for (_hash, entry) in self.index.iter() {
            tt.resize_for_copy(entry.size);
            self.file.read_exact_at(tt.as_mut_buf(), entry.offset)?;
            if tt.is_valid_for_copy() {
                println!("{}", tt);
                tmp.write_all(tt.as_buf())?;
            }
            tt.reset();
        }
        tmp.flush()?;
        tmp.sync_data()?;
        let dst_pb = self.pack_path();
        fs::rename(&tmp_pb, &dst_pb)?;
        Ok(())
    }

    pub fn commit_object(&mut self, top: &TubTop, obj: NewObj) -> io::Result<bool>
    {
        if let Some(_entry) = self.index.get(&top.hash()) {
            Ok(false)  // Already in object store
        }
        else {
            let entry = Entry::new(top.size(), self.offset);
            match obj {
                NewObj::File(tmp) => {
                    if top.is_small() {
                        assert!(tmp.is_small());
                        return self.commit_object(top, NewObj::Mem(&tmp.into_data()));
                    }
                    assert!(top.is_large());
                    self.finalize_tmp(tmp, &top.hash())?;
                    self.file.write_all(top.as_buf())?;
                    self.offset += top.len() as u64;
                }
                NewObj::Mem(data) => {
                    if top.is_large() {
                        let mut tmp = self.allocate_tmp()?;
                        tmp.write_all(data)?;
                        return self.commit_object(top, NewObj::File(tmp));
                    }
                    assert!(top.is_small());
                    self.file.write_all_vectored(&mut [
                        io::IoSlice::new(top.as_buf()),
                        io::IoSlice::new(data),
                    ])?;
                    self.offset += (top.len() + data.len()) as u64;
                }
            }
            self.index.insert(top.hash(), entry);
            Ok(true)
        }
    }

    pub fn add_object(&mut self, data: &[u8]) -> io::Result<(TubTop, bool)> {
        // FIXME: no reason not to handle the large object case as well
        let mut tt = TubTop::new();
        tt.hash_data(data);
        let new = self.commit_object(&tt, NewObj::Mem(data))?;
        Ok((tt, new))
    }

    pub fn get_object(&mut self, id: &TubHash, verify: bool) -> io::Result<Option<Vec<u8>>>
    {
        if let Some(entry) = self.index.get(id) {
            let mut buf = vec![0_u8; entry.size as usize];
            self.file.read_exact_at(&mut buf, entry.data_offset())?;
            if verify && id != &hash(&buf).hash {
                eprintln!("{} is corrupt", db32enc_str(id));
                self.delete_object(id)?;
            }
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
            if id != &hash(buf).hash {
                eprintln!("{} is corrupt", db32enc_str(id));
                self.delete_object(id)?;
            }
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
            self.file.write_all_vectored(&mut [
                io::IoSlice::new(hash),
                io::IoSlice::new(&(0_u64).to_le_bytes()),
                // This makes the tombstone verifiable
                io::IoSlice::new(&hash_tombstone(hash)),
            ])?;
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
    use crate::util::*;
    use crate::helpers::TestTempDir;

    #[test]
    fn test_entry() {
        let entry = Entry::new(1, 7);
        assert_eq!(entry.size, 1);
        assert_eq!(entry.offset, 7);
        assert_eq!(entry.data_offset(), 7 + data_offset(1));
        assert_eq!(entry.is_large(), false);
        assert_eq!(entry.is_small(), true);

        let entry = Entry::new(LEAF_SIZE + 1, 11);
        assert_eq!(entry.size, LEAF_SIZE + 1);
        assert_eq!(entry.offset, 11);
        assert_eq!(entry.data_offset(), 11 + data_offset(LEAF_SIZE + 1));
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
    fn test_push_repack_path() {
        let id = [0_u8; TUB_ID_LEN];
        let mut pb = PathBuf::new();
        push_repack_path(&mut pb, &id);
        assert_eq!(pb.as_os_str(), "bathtub.333333333333333333333333");
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
    fn test_store() {

    }

    #[test]
    fn test_store_large() {
        let (_tmp, store) = Store::new_tmp();
        let id = random_hash();
        assert!(store.open_large(&id).is_err());
        assert!(store.remove_large(&id).is_err());
        //assert!(store.open(&id).is_err());
    }
}

