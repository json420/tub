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
use crate::protocol::{hash, RootInfo};
use crate::dbase32::{db32enc_str, Name2Iter};
use crate::util::random_id;
use crate::leaf_io::{Object, LeafReader, new_leaf_buf, TubTop, TmpObject};


macro_rules! other_err {
    ($msg:literal) => {
        Err(io::Error::new(io::ErrorKind::Other, $msg))
    }
}


/// An entry in the HashMap index.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Entry {
    offset: u64,
    size: u64,
}

impl Entry {
    pub fn is_small(&self) -> bool {
        ! self.is_large()
    }

    pub fn is_large(&self) -> bool {
        self.size > LEAF_SIZE
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
            Store {path: pb, file: file, index: HashMap::new()}
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

    pub fn import_file(&mut self, file: File) -> io::Result<(RootInfo, bool)>
    {
        let mut reader = LeafReader::new(file);
        let mut tmp = self.allocate_tmp()?;
        let mut buf = new_leaf_buf();
        while let Some(_info) = reader.read_next_leaf(&mut buf)? {
            tmp.write_leaf(&buf)?;
        }
        let root = reader.hash_root();
        let new = match root.small() {
            true => {
                let data = tmp.into_data();
                self.add_small_object(&root, &data)?
            }
            false => {
                self.finalize_tmp(tmp, &root.hash)?;
                self.add_large_object_meta(&root)?
            }
        };
        Ok((root, new))
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
                    Object::new(file, entry.size, entry.offset + HEADER_LEN as u64)
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
        // FIXME: We should truncate off the end of the file any partially
        // written object we find.  Basically if after the last valid object
        // is read there is still additional data, but not enough to make a
        // valid object entry... then a prior object append operation was
        // interrupted, and so we should discard the invalid partial object.
        self.index.clear();
        self.file.seek(io::SeekFrom::Start(0))?;

        let mut offset: u64 = 0;
        let mut header: HeaderBuf = [0_u8; HEADER_LEN];
        loop {
            if let Err(_) = self.file.read_exact(&mut header) {
                break;
            }
            let id: TubHash = header[0..30].try_into().expect("oops");
            let size = u64::from_le_bytes(
                header[30..38].try_into().expect("oops")
            );

            if size == 0 {
                // Deletion tombstone
                if self.index.remove(&id) == None {
                    panic!("{} not in index but tombstone found", db32enc_str(&id));
                }
            }
            else {
                let entry = Entry {offset: offset, size: size};
                self.index.insert(id, entry);
                offset += HEADER_LEN as u64;
                if size <= LEAF_SIZE {
                    // Only small objects are in self.file
                    offset += size;
                    self.file.seek(io::SeekFrom::Current(size as i64))?;
                }
            }
        }
        Ok(())
    }

    pub fn add_small_object(&mut self, root: &RootInfo, data: &[u8]) -> io::Result<bool>
    {
        assert!(root.small());
        if let Some(_entry) = self.index.get(&root.hash) {
            Ok(false)  // Already in object store
        }
        else {
            let entry = Entry {
                offset: self.file.stream_position().unwrap(),
                size: data.len() as u64,
            };
            self.file.write_all_vectored(&mut [
                io::IoSlice::new(&root.hash),
                io::IoSlice::new(&entry.size.to_le_bytes()),
                io::IoSlice::new(data),
            ])?;
            self.index.insert(root.hash.clone(), entry);
            Ok(true)
        }
    }

    pub fn add_large_object_meta(&mut self, root: &RootInfo) -> io::Result<bool>
    {
        assert!( !root.small());
        if let Some(_entry) = self.index.get(&root.hash) {
            Ok(false)  // Already in object store
        }
        else {
            let entry = Entry {
                offset: self.file.stream_position()?,
                size: root.size,
            };
            self.file.write_all_vectored(&mut [
                io::IoSlice::new(&root.hash),
                io::IoSlice::new(&root.size.to_le_bytes()),
                // NOTE: we write just the header for large object, no data
            ])?;
            self.index.insert(root.hash.clone(), entry);
            Ok(true)
        }
    }

    pub fn commit_object(&mut self, top: &TubTop, obj: NewObj) -> io::Result<bool>
    {
        if let Some(_entry) = self.index.get(&top.hash()) {
            Ok(false)  // Already in object store
        }
        else {
            let entry = Entry {
                offset: self.file.stream_position()?,
                size: top.size(),
            };
            match obj {
                NewObj::File(tmp) => {
                    if top.is_small() {
                        assert!(tmp.is_small());
                        return self.commit_object(top, NewObj::Mem(&tmp.into_data()));
                    }
                    assert!(top.is_large());
                    self.finalize_tmp(tmp, &top.hash())?;
                    self.file.write_all(top.as_buf())?;
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
                }
            }
            self.index.insert(top.hash(), entry);
            Ok(true)
        }
    }

    pub fn write_small_object(&mut self, top: &TubTop, data: &[u8]) -> io::Result<bool>
    {
        assert!(top.is_small());
        if let Some(_entry) = self.index.get(&top.hash()) {
            Ok(false)  // Already in object store
        }
        else {
            let entry = Entry {
                offset: self.file.stream_position().unwrap(),
                size: data.len() as u64,
            };
            self.file.write_all_vectored(&mut [
                io::IoSlice::new(top.as_buf()),
                io::IoSlice::new(data),
            ])?;
            self.index.insert(top.hash(), entry);
            Ok(true)
        }
    }

    pub fn add_object(&mut self, data: &[u8]) -> io::Result<(RootInfo, bool)> {
        // FIXME: no reason not to handle the large object case as well
        let mut tt = TubTop::new();
        tt.hash_data(data);
        let root = hash(data);
        assert_eq!(tt.hash(), root.hash);
        let new = self.add_small_object(&root, data)?;
        Ok((tt.as_root_info(), new))
    }

    pub fn get_object(&mut self, id: &TubHash, verify: bool) -> io::Result<Option<Vec<u8>>>
    {
        if let Some(entry) = self.index.get(id) {
            let mut buf = vec![0_u8; entry.size as usize];
            let offset = entry.offset + (HEADER_LEN as u64);
            self.file.read_exact_at(&mut buf, offset)?;
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
            let offset = entry.offset + (HEADER_LEN as u64);
            self.file.read_exact_at(buf, offset)?;
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
    use crate::dbase32::{db32enc_str, Name2Iter};
    use crate::util::*;
    use crate::helpers::TestTempDir;

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

