//! Low-level object store.

/* FIXME:

Ideally we should do filesystem IO relative to an open directory descriptor.
However, this is not currently supported is the Rust standard library.

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
use crate::protocol::hash;
use crate::dbase32::{db32enc, db32enc_str, Name2Iter};
use crate::util::{fadvise_random, fadvise_sequential};


const DOTDIR: &str = ".bathtub_db";
const PACKFILE: &str = "bathtub.db";
const OBJECTDIR: &str = "objects";
const PARTIALDIR: &str = "partial";
const TMPDIR: &str = "tmp";
const README: &str = "README.txt";  // The REAMDE file

static README_CONTENTS: &[u8] = b"Hello from Bathtub  DB!

What's even more relaxing than a Couch?  A Bathtub!
";


macro_rules! other_err {
    ($msg:literal) => {
        Err(io::Error::new(io::ErrorKind::Other, $msg));
    }
}


/// Represents an object open for reading (both large and small objects)
#[derive(Debug)]
pub struct Object {
    offset: OffsetSize,
    size: ObjectSize,
    id: ObjectID,
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


type Index = HashMap<ObjectID, Entry>;


/// Find the control directory by walking up the path.
pub fn find_dotdir(pb: &mut PathBuf) -> bool
{
    loop {
        pb.push(DOTDIR);
        if pb.is_dir() {
            return true;
        }
        pb.pop();
        if !pb.pop() {
            return false;
        }
    }
}


pub fn find_store(path: &Path) -> io::Result<Store>
{
    let mut pb = path.canonicalize()?;
    loop {
        pb.push(DOTDIR);
        if pb.is_dir() {
            return Store::new(pb);
        }
        pb.pop();
        if !pb.pop() {
            return other_err!("cannot find control directroy");
        }
    }
}



/// Initialize a store layout in an empty directory.
pub fn init_store(pb: &mut PathBuf) -> io::Result<()>
{
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
    fs::create_dir(pb.as_path());
    pb.pop();

    // tmp directory:
    pb.push(TMPDIR);
    fs::create_dir(pb.as_path());
    pb.pop();

    // REAMDE file  :-)
    pb.push(README);
    let mut f = fs::File::create(pb.as_path())?;
    f.write_all(README_CONTENTS)?;
    pb.pop();

    Ok(())
}


/// Creates the `".bathtub_db"` directory, calls `init_store()`.
pub fn init_tree(pb: &mut PathBuf) -> io::Result<Store>
{
    pb.push(DOTDIR);
    fs::create_dir(pb.as_path())?;
    init_store(pb)?;
    Store::new(pb.as_path())
}


fn push_object_path(pb: &mut PathBuf, id: &ObjectID) {
    pb.push(OBJECTDIR);
    let sid = db32enc_str(id);
    let (prefix, suffix) = sid.split_at(2);
    pb.push(prefix);
    pb.push(suffix);
}

fn push_partial_path(pb: &mut PathBuf, id: &ObjectID) {
    pb.push(PARTIALDIR);
    pb.push(db32enc_str(id));
}

fn push_tmp_path(pb: &mut PathBuf, key: &AbstractID) {
    pb.push(TMPDIR);
    pb.push(db32enc_str(key));
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
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self>
            where PathBuf: From<P> 
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
        let store = Store::new(tmp.path()).unwrap();
        (tmp, store)
    }

    /// Returns clone of self.path
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Builds canonical large file path.
    pub fn object_path(&self, id: &ObjectID) -> PathBuf {
        let mut pb = self.path();
        push_object_path(&mut pb, id);
        pb
    }

    /// Builds canonical partial large file path.
    ///
    /// A "partial" object is an object whose hash/ID is known, but not all the
    /// its leaves are present in this store.
    pub fn partial_path(&self, id: &ObjectID) -> PathBuf {
        let mut pb = self.path();
        push_partial_path(&mut pb, id);
        pb
    }

    fn open_large(&self, id: &ObjectID) -> io::Result<fs::File> {
        File::open(self.object_path(id))
    }

    fn remove_large(&self, id: &ObjectID) -> io::Result<()> {
        fs::remove_file(self.object_path(id))
    }

    pub fn open(&self, id: &ObjectID) -> io::Result<Object> {
        other_err!("oh no!")
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn keys(&self) -> Vec<ObjectID> {
        Vec::from_iter(self.index.keys().cloned())
    }

    pub fn sync_data(&mut self) {
        self.file.flush().expect("nope");
        self.file.sync_data().expect("nope");
    }

    pub fn reindex(&mut self, check: bool) {
        // FIXME: We should truncate off the end of the file any partially
        // written object we find.  Basically if after the last valid object
        // is read there is still additional data, but not enough to make a
        // valid object entry... then a prior object append operation was
        // interrupted, and so we should discard the invalid partial object.
        self.index.clear();
        fadvise_sequential(&self.file);
        let mut offset: OffsetSize = 0;
        let mut buf = vec![0_u8; 4096];

        self.file.seek(io::SeekFrom::Start(0)).unwrap();
        let mut header: HeaderBuf = [0_u8; HEADER_LEN];
        loop {
            if let Err(_) = self.file.read_exact(&mut header) {
                break;
            }
            let id: ObjectID = header[0..30].try_into().expect("oops");
            let size = u64::from_le_bytes(
                header[30..38].try_into().expect("oops")
            );
            if size > 0 {
                let entry = Entry {
                    offset: offset,
                    size: size,
                };
                self.index.insert(id, entry);

                offset += HEADER_LEN as ObjectSize + size;

                if check {
                    buf.resize(size as usize, 0);
                    let s = &mut buf[0..(size as usize)];
                    self.file.read_exact(s).expect("oops");
                    if id != hash(s) {
                        panic!("hash does not equal expected");
                    }
                }
                else {
                    self.file.seek(io::SeekFrom::Current(size as i64)).expect("oops");
                }
            }
            else {
                //println!("Tombstone {}", db32enc_str(&id));
                if self.index.remove(&id) == None {
                    panic!("{} not in index but tombstone found", db32enc_str(&id));
                }
            }
        }
        fadvise_random(&self.file);
    }

    pub fn add_object(&mut self, data: &[u8]) -> (ObjectID, bool) {
        let id = hash(data);
        if let Some(entry) = self.index.get(&id) {
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

    pub fn get_object(&mut self, id: &ObjectID, verify: bool) -> Option<Vec<u8>> {
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

    pub fn delete_object(&mut self, id: &ObjectID) -> bool {
        /*  Remove an object from the Store.

        This writes a tombstone to the pack file and then, in the large object
        case, remove the corresponding o/AA/AAA... file.  When the next repack
        occurs, the object entry in the pack file and the tombstone will be
        removed (not copied into the new pack file).
        */
        if let Some(entry) = self.index.get(id) {
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
    use std::fs::File;
    use crate::dbase32::{db32enc_str, Name2Iter};
    use crate::util::*;
    use crate::helpers::TestTempDir;

    #[test]
    fn test_find_dotdir() {
        let mut pb = PathBuf::new();
        assert!(! find_dotdir(&mut pb));
        assert_eq!(pb.as_os_str(), "");

        let tmp = TestTempDir::new();
        tmp.makedirs(&["foo", "bar"]);
        tmp.mkdir(&["foo", DOTDIR]);
        let mut pb = PathBuf::from(tmp.path());
        pb.push("foo");
        pb.push("bar");
        assert!(find_dotdir(&mut pb));
        assert!(pb.as_path().ends_with("foo/.bathtub_db"));
    }

    #[test]
    fn test_push_object_path() {
        let id = [0_u8; OBJECT_ID_LEN];
        let mut pb = PathBuf::new();
        push_object_path(&mut pb, &id);
        assert_eq!(pb.as_os_str(),
            "objects/33/3333333333333333333333333333333333333333333333"
        );
    }

    #[test]
    fn test_push_partial_path() {
        let id = [0_u8; OBJECT_ID_LEN];
        let mut pb = PathBuf::new();
        push_partial_path(&mut pb, &id);
        assert_eq!(pb.as_os_str(),
            "partial/333333333333333333333333333333333333333333333333"
        );
    }

    #[test]
    fn test_push_tmp_path() {
        let key = [0_u8; ABSTRACT_ID_LEN];
        let mut pb = PathBuf::new();
        push_tmp_path(&mut pb, &key);
        assert_eq!(pb.as_os_str(),
            "tmp/333333333333333333333333"
        );
    }

    #[test]
    fn test_init_tree() {
        let tmp = TestTempDir::new();
        let mut pb = PathBuf::from(tmp.path());
        init_tree(&mut pb);
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
        let mut pb = PathBuf::from(tmp.path());
        init_store(&mut pb);
        let mut expected = vec![OBJECTDIR, PARTIALDIR, TMPDIR, README];
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
        let (tmp, mut store) = Store::new_tmp();

        assert_eq!(store.len(), 0);
        let empty: Vec<ObjectID> = vec![];
        assert_eq!(store.keys(), empty);

        let rid = random_object_id();
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
            "OK5UTJXH6H3Q9DU7EHY9LEAN8P6TPY553SIGLQH5KAXEG6EN"
        );
        assert!(new);
    }

    #[test]
    fn test_store_large() {
        let (tmp, mut store) = Store::new_tmp();
        let id = random_object_id();
        assert!(store.open_large(&id).is_err());
        assert!(store.remove_large(&id).is_err());
        assert!(store.open(&id).is_err());
    }
}

