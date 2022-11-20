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
use crate::leaf_io::{LeafReader, new_leaf_buf};


macro_rules! other_err {
    ($msg:literal) => {
        Err(io::Error::new(io::ErrorKind::Other, $msg))
    }
}


/// Represents an object open for reading (both large and small objects)
#[derive(Debug)]
pub struct Object {
    offset: OffsetSize,
    size: ObjectSize,
    id: TubHash,
    rfile: fs::File,
}

impl Object {

}


/// An entry in the HashMap index.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Entry {
    offset: OffsetSize,
    size: ObjectSize,
}


type Index = HashMap<TubHash, Entry>;


pub fn find_store(path: &Path) -> io::Result<Store>
{
    let mut pb = path.canonicalize()?;
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
    let mut pb = path.canonicalize()?;

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

fn push_tmp_path(pb: &mut PathBuf, key: &TubID) {
    pb.push(TMPDIR);
    pb.push(db32enc_str(key));
}

#[derive(Debug)]
pub struct TmpObject {
    pub id: TubID,
    pub path: PathBuf,
    buf: Option<Vec<u8>>,
    file: Option<File>,
}

impl TmpObject {
    pub fn new(id: TubID, path: PathBuf) -> io::Result<Self>
    {
        Ok(TmpObject {
            id: id,
            path: path,
            buf: None,
            file: None,
        })
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
        let pb = path.canonicalize()?;

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

    pub fn tmp_path(&self, id: &TubID) -> PathBuf {
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

    pub fn import_file(&mut self, file: File) -> io::Result<RootInfo>
    {
        let mut reader = LeafReader::new(file);
        let mut tmp = self.allocate_tmp()?;
        let mut buf = new_leaf_buf();
        while let Some(info) = reader.read_next_leaf(&mut buf)? {
            tmp.write_leaf(&buf)?;
        }
        let root = reader.hash_root();
        if root.is_small() {
            // Super lame, fix ASAP!
            let mut buf:Vec<u8> = Vec::with_capacity(root.size as usize);
            buf.resize(root.size as usize, 0);
        }
        else {
            self.finalize_tmp(tmp, &root.hash)?;
            self.add_large_object_meta(&root)?;
        }
        Ok(root)
    }

    fn open_large(&self, id: &TubHash) -> io::Result<fs::File> {
        File::open(self.object_path(id))
    }

    fn remove_large(&self, id: &TubHash) -> io::Result<()> {
        fs::remove_file(self.object_path(id))
    }

    pub fn open(&self, _id: &TubHash) -> io::Result<Object> {
        other_err!("oh no!")
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

        let mut offset: OffsetSize = 0;
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
                offset += HEADER_LEN as ObjectSize;
                if size <= LEAF_SIZE {
                    // Only small objects are in self.file
                    offset += size;
                    self.file.seek(io::SeekFrom::Current(size as i64))?;
                }
            }
        }
        Ok(())
    }

    pub fn add_large_object_meta(&mut self, root: &RootInfo) -> io::Result<()>
    {
        let entry = Entry {
            offset: self.file.stream_position()?,
            size: root.size,
        };
        self.file.write_all_vectored(&mut [
            io::IoSlice::new(&root.hash),
            io::IoSlice::new(&root.size.to_le_bytes()),
        ])?;
        self.index.insert(root.hash.clone(), entry);
        Ok(())
    }

    pub fn add_object(&mut self, data: &[u8]) -> (TubHash, bool) {
        let id = hash(data);
        if let Some(_entry) = self.index.get(&id) {
            return (id, false);  // Already in object store
        }
        let entry = Entry {
            offset: self.file.stream_position().unwrap(),
            size: data.len() as ObjectSize,
        };
        self.file.write_all_vectored(&mut [
            io::IoSlice::new(&id),
            io::IoSlice::new(&entry.size.to_le_bytes()),
            io::IoSlice::new(data),
        ]).expect("object append failed");
        self.index.insert(id, entry);
        (id, true)
    }

    pub fn get_object(&mut self, id: &TubHash, verify: bool) -> Option<Vec<u8>> {
        if let Some(entry) = self.index.get(id) {
            let mut buf = vec![0_u8; entry.size as usize];
            let s = &mut buf[0..entry.size as usize];
            let offset = entry.offset + (HEADER_LEN as ObjectSize);
            self.file.read_exact_at(s, offset).expect("oops");
            if verify && id != &hash(s) {
                eprintln!("{} is corrupt", db32enc_str(id));
                self.delete_object(id);
            }
            return Some(buf);
        }
        None
    }

    pub fn delete_object(&mut self, id: &TubHash) -> bool {
        /*  Remove an object from the Store.

        This writes a tombstone to the pack file and then, in the large object
        case, remove the corresponding o/AA/AAA... file.  When the next repack
        occurs, the object entry in the pack file and the tombstone will be
        removed (not copied into the new pack file).
        */
        if let Some(_entry) = self.index.get(id) {
            eprintln!("Deleting {}", db32enc_str(id));
            self.file.write_all_vectored(&mut [
                io::IoSlice::new(id),
                io::IoSlice::new(&(0_u64).to_le_bytes()),
            ]).expect("failed to write tombstone");
            self.index.remove(id);
            //FIXME: In large object case, also delete object file
            true
        }
        else {
            false  // id not in this store
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
        let (_tmp, mut store) = Store::new_tmp();

        assert_eq!(store.len(), 0);
        let empty: Vec<TubHash> = vec![];
        assert_eq!(store.keys(), empty);

        let rid = random_hash();
        assert_eq!(store.get_object(&rid, false), None);
        assert_eq!(store.get_object(&rid, true), None);
        assert_eq!(store.delete_object(&rid), false);
        assert_eq!(store.len(), 0);
        assert_eq!(store.keys(), empty);

        let data = random_small_object();
        let (id, new) = store.add_object(&data);
        assert!(new);
        assert_eq!(store.len(), 1);
        assert_eq!(store.keys(), vec![id]);
        assert_eq!(store.get_object(&id, false).unwrap(), data);
        assert_eq!(store.get_object(&id, true).unwrap(), data);

        // Re-add object:
        let (id2, new) = store.add_object(&data);
        assert_eq!(id2, id);
        assert!(!new);
        assert_eq!(store.len(), 1);
        assert_eq!(store.keys(), vec![id]);
        assert_eq!(store.get_object(&id, false).unwrap(), data);
        assert_eq!(store.get_object(&id, true).unwrap(), data);

        assert_eq!(store.get_object(&rid, false), None);
        assert_eq!(store.get_object(&rid, true), None);

        // Delete object:
        assert_eq!(store.delete_object(&id), true);
        assert_eq!(store.len(), 0);
        assert_eq!(store.delete_object(&id), false);

        // Known test vector or something like that
        let (id3, new) = store.add_object(b"Federation44");
        assert_eq!(&db32enc_str(&id3),
            "TDJGJI47CFS53WQWE7K77R8GJVIAE9KB6465SPUV6NDYPVKA"
        );
        assert!(new);
    }

    #[test]
    fn test_store_large() {
        let (_tmp, store) = Store::new_tmp();
        let id = random_hash();
        assert!(store.open_large(&id).is_err());
        assert!(store.remove_large(&id).is_err());
        assert!(store.open(&id).is_err());
    }
}

