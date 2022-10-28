use std::fs::File;
use std::path::Path;
use std::os::unix::fs::FileExt;
use std::io::prelude::*;
use std::collections::HashMap;
use std::io::IoSlice;
use std::io::SeekFrom;

use tempfile::TempDir;

use crate::base::*;
use crate::protocol::hash;
use crate::dbase32::{db32enc, db32enc_str};



#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Entry {
    offset: OffsetSize,
    size: ObjectSize,
}

type Index = HashMap<ObjectID, Entry>;


#[derive(Debug)]
pub struct Store {
    pub file: File,
    pub index: Index,
}

// FIXME: for multithread, Store needs to be wrapped in Arc<Mutex<>>
impl Store {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let index: Index = HashMap::new();
        let file = File::options()
            .append(true)
            .read(true)
            .create(true)
            .open(path).expect("could not open pack file");
        Store {
            file: file,
            index: index,
        }
    }

    pub fn new_tmp() -> (TempDir, Self) {
        let tmp = TempDir::new().unwrap();
        let mut pb = tmp.path().to_path_buf();
        pb.push("temp.btdb");
        (tmp, Store::new(pb))
    }

    pub fn len(&mut self) -> usize {
        self.index.len()
    }

    pub fn keys(&mut self) -> Vec<ObjectID> {
        Vec::from_iter(self.index.keys().cloned())
    }

    pub fn reindex(&mut self, check: bool) {
        // FIXME: We should truncate off the end of the file any partially
        // written object we find.  Basically if after the last valid object
        // is read there is still additional data, but not enough to make a
        // valid object entry... then a prior object append operation was
        // interrupted, and so we should discard the invalid partial object.
        self.index.clear();

        let mut offset: OffsetSize = 0;
        let mut buf = vec![0_u8; 4096];

        self.file.seek(SeekFrom::Start(0)).unwrap();
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
                    self.file.seek(SeekFrom::Current(size as i64)).expect("oops");
                }
            }
            else {
                //println!("Tombstone {}", db32enc_str(&id));
                if self.index.remove(&id) == None {
                    panic!("{} not in index but tombstone found", db32enc_str(&id));
                }
            }
        }
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
            offset: self.file.stream_position().unwrap(),
            size: data.len() as ObjectSize,
        };
        self.file.write_all_vectored(&mut [
            IoSlice::new(&id),
            IoSlice::new(&entry.size.to_le_bytes()),
            IoSlice::new(data),
        ]).expect("object append failed");
        self.file.flush().expect("nope");
        self.index.insert(id, entry);
        (id, true)
    }

    pub fn get_object(&mut self, id: &ObjectID, verify: bool) -> Option<Vec<u8>> {
        if let Some(entry) = self.get(id) {
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
    use crate::dbase32::db32enc_str;
    use crate::util::*;

    #[test]
    fn test_store() {
        let tmp = TempDir::new().unwrap();
        let mut pb = tmp.path().to_path_buf();
        pb.push("example.btdb");

        let mut store = Store::new(pb);
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
    fn test_get() {
        let tmp = TempDir::new().unwrap();
        let mut pb = tmp.path().to_path_buf();
        pb.push("example.btdb");
        let mut store = Store::new(pb);
        let id = random_object_id();
        assert_eq!(store.get(&id), None);
        let entry = Entry {size: 3, offset: 5};
        assert_eq!(store.index.insert(id.clone(), entry.clone()), None);
        assert_eq!(store.get(&id), Some(entry));
    }

}

