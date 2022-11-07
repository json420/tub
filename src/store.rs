use std::fs::{File, create_dir};
use std::path::{Path, PathBuf};
use std::os::unix::fs::FileExt;
use std::io::prelude::*;
use std::collections::HashMap;
use std::io::IoSlice;
use std::io;
use std::io::SeekFrom;
use std::fs;

use tempfile::TempDir;
use openat;

use crate::base::*;
use crate::protocol::hash;
use crate::dbase32::{db32enc, db32enc_str, Name2Iter};
use crate::util::{fadvise_random, fadvise_sequential};



const PACKFILE: &str = "bathtub.db";
const OBJECTDIR: &str = "objects";
const DIRMODE: u32 = 0o770;
const FILEMODE: u32 = 0o660;
const README: &str = "README.txt";

static README_CONTENTS: &[u8] = b"Hello from Bathtub  DB!

What's even more relaxing than a Couch?  A Bathtub!
";


/// Represents an object open for reading (both large and small objects)
#[derive(Debug)]
pub struct Object {
    offset: OffsetSize,
    size: ObjectSize,
    id: ObjectID,
    rfile: File,
}

impl Object {

}






#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Entry {
    offset: OffsetSize,
    size: ObjectSize,
}

type Index = HashMap<ObjectID, Entry>;


#[derive(Debug)]
pub struct Store {
    dir: openat::Dir,
    odir: openat::Dir,
    afile: File,
    rfile: File,
    index: Index,
}


/// Initialize directories and files that Store uses
pub fn init_store_layout(dir: &openat::Dir) -> io::Result<()> {
    dir.create_dir(OBJECTDIR, DIRMODE)?;
    for name in Name2Iter::new() {
        let pb: PathBuf = [OBJECTDIR, &name].iter().collect();
        println!("{:?}", pb);
        dir.create_dir(&pb, DIRMODE)?;
    }
    dir.create_file(PACKFILE, FILEMODE)?;
    Ok(())
}


/// Initialize a store layout in an empty directory.
pub fn init_store<P: AsRef<Path>>(dir: P) -> io::Result<()> 
    where PathBuf: From<P>
    {
    let mut pb = PathBuf::from(dir);
    pb.push(OBJECTDIR);
    create_dir(&pb)?;
    for name in Name2Iter::new() {
        pb.push(name);
        create_dir(&pb)?;
        pb.pop();
    }
    pb.pop();
    pb.push(README);
    let mut f = fs::File::create(&pb)?;
    f.write_all(README_CONTENTS);
    Ok(())
}


// FIXME: for multithread, Store needs to be wrapped in Arc<Mutex<>>
impl Store {
    pub fn new(dir: openat::Dir) -> Self {
        dir.create_dir(OBJECTDIR, DIRMODE);
        let odir = dir.sub_dir(OBJECTDIR).unwrap();
        let afile = dir.append_file(PACKFILE, FILEMODE).unwrap();
        let rfile = dir.open_file(PACKFILE).unwrap();
        Store {
            dir: dir,
            odir: odir,
            afile: afile,
            rfile: rfile,
            index: HashMap::new(),
        }
    }

    pub fn new_tmp() -> (TempDir, Self) {
        let tmp = TempDir::new().unwrap();
        let dir = openat::Dir::open(tmp.path()).unwrap();
        (tmp, Store::new(dir))
    }

    pub fn new_cwd() -> Self {
        let dir = openat::Dir::open(".").unwrap();
        Store::new(dir)
    }

    fn open_large(&self, id: &ObjectID) -> io::Result<File> {
        self.odir.open_file(db32enc_str(id))
    }

    fn remove_large(&self, id: &ObjectID) -> io::Result<()> {
        self.odir.remove_file(db32enc_str(id))
    }

    fn new_unnamed_file(&self) -> io::Result<File> {
        self.odir.new_unnamed_file(0o0400)
    }

    fn link_file_at(&self, file: &File, id: &ObjectID) -> io::Result<()> {
        self.odir.link_file_at(file, db32enc_str(id))
    }

    pub fn open(&self, id: &ObjectID) -> io::Result<Object> {
        Err(io::Error::new(io::ErrorKind::Other, "oh no!"))
    }

    pub fn len(&mut self) -> usize {
        self.index.len()
    }

    pub fn keys(&mut self) -> Vec<ObjectID> {
        Vec::from_iter(self.index.keys().cloned())
    }

    pub fn sync_data(&mut self) {
        self.afile.flush().expect("nope");
        self.afile.sync_data().expect("nope");
    }

    pub fn reindex(&mut self, check: bool) {
        // FIXME: We should truncate off the end of the file any partially
        // written object we find.  Basically if after the last valid object
        // is read there is still additional data, but not enough to make a
        // valid object entry... then a prior object append operation was
        // interrupted, and so we should discard the invalid partial object.
        self.index.clear();
        fadvise_sequential(&self.rfile);
        let mut offset: OffsetSize = 0;
        let mut buf = vec![0_u8; 4096];

        self.rfile.seek(SeekFrom::Start(0)).unwrap();
        let mut header: HeaderBuf = [0_u8; HEADER_LEN];
        loop {
            if let Err(_) = self.rfile.read_exact(&mut header) {
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
                    self.rfile.read_exact(s).expect("oops");
                    if id != hash(s) {
                        panic!("hash does not equal expected");
                    }
                }
                else {
                    self.rfile.seek(SeekFrom::Current(size as i64)).expect("oops");
                }
            }
            else {
                //println!("Tombstone {}", db32enc_str(&id));
                if self.index.remove(&id) == None {
                    panic!("{} not in index but tombstone found", db32enc_str(&id));
                }
            }
        }
        fadvise_random(&self.rfile);
    }

    fn get(&self, id: &ObjectID) -> Option<Entry> {
        if let Some(val) = self.index.get(id) {
            Some(val.clone())
        }
        else {
            None
        }
    }

    pub fn add_object(&mut self, data: &[u8]) -> (ObjectID, bool) {
        let id = hash(data);
        if let Some(entry) = self.index.get(&id) {
            return (id, false);  // Already in object store
        }
        let entry = Entry {
            offset: self.afile.stream_position().unwrap(),
            size: data.len() as ObjectSize,
        };
        self.afile.write_all_vectored(&mut [
            IoSlice::new(&id),
            IoSlice::new(&entry.size.to_le_bytes()),
            IoSlice::new(data),
        ]).expect("object append failed");
        self.index.insert(id, entry);
        (id, true)
    }

    pub fn get_object(&mut self, id: &ObjectID, verify: bool) -> Option<Vec<u8>> {
        if let Some(entry) = self.index.get(id) {
            let mut buf = vec![0_u8; entry.size as usize];
            let s = &mut buf[0..entry.size as usize];
            let offset = entry.offset + (HEADER_LEN as ObjectSize);
            self.rfile.read_exact_at(s, offset).expect("oops");
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
            self.afile.write_all_vectored(&mut [
                IoSlice::new(id),
                IoSlice::new(&(0_u64).to_le_bytes()),
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
    fn test_init_store() {
        let tmp = TestTempDir::new();
        init_store(tmp.path());
        let mut expected = vec![OBJECTDIR, README];
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

        let mut wfile = store.new_unnamed_file().unwrap();
        wfile.write(b"hello, world");
        assert!(store.link_file_at(&mut wfile, &id).is_ok());
        assert!(store.remove_large(&id).is_ok());
        assert!(store.open_large(&id).is_err());
    }

    #[test]
    fn test_get() {
        let (tmp, mut store) = Store::new_tmp();
        let id = random_object_id();
        assert_eq!(store.get(&id), None);
        let entry = Entry {size: 3, offset: 5};
        assert_eq!(store.index.insert(id.clone(), entry.clone()), None);
        assert_eq!(store.get(&id), Some(entry));
    }

}

