use std::fs::File;
use std::path::Path;
use std::os::unix::fs::FileExt;
use std::io::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::io::IoSlice;
use std::io::SeekFrom;

use crate::base::*;
use crate::protocol::hash;
use crate::dbase32::db32enc;



#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Entry {
    offset: OffsetSize,
    size: ObjectSize,
}

type Index = Arc<Mutex<HashMap<ObjectID, Entry>>>;


#[derive(Debug)]
pub struct Store {
    pub file: File,
    pub index: Index,
}

impl Store {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let index: Index = Arc::new(Mutex::new(HashMap::new()));
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

    pub fn len(&mut self) -> usize {
        self.index.lock().unwrap().len()
    }

    pub fn keys(&mut self) -> Vec<ObjectID> {
        Vec::from_iter(self.index.lock().unwrap().keys().cloned())
    }

    pub fn reindex(&mut self, check: bool) {
        let mut index = self.index.lock().unwrap();
        index.clear();

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
            //println!("{} {}\t{}", encode(&id), offset, size);

            let entry = Entry {
                offset: offset,
                size: size,
            };
            index.insert(id, entry);

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
            };
            
        }
    }

    fn get(&self, id: &ObjectID) -> Option<Entry> {
        let index = self.index.lock().unwrap();
        if let Some(val) = index.get(id) {
            Some(val.clone())
        }
        else {
            None
        }
    }

    pub fn add_object(&mut self, data: &[u8]) -> (ObjectID, bool) {
        let id = hash(data);
        let mut index = self.index.lock().unwrap();
        if let Some(entry) = index.get(&id) {
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
        index.insert(id, entry);
        (id, true)
    }

    pub fn get_object(&mut self, id: &ObjectID, verify: bool) -> Option<Entry> {
        if let Some(entry) = self.get(id) {
            let mut buf = vec![0_u8; entry.size as usize];
            assert_eq!(buf.len(), entry.size as usize);
            let s = &mut buf[0..entry.size as usize];
            let offset = entry.offset + (HEADER_LEN as ObjectSize);
            self.file.read_exact_at(s, offset).expect("oops");
            if verify && id != &hash(s) {
                /*  FIXME: When hash doesn't match, we should remove from index
                    and then either (1) in the small object case append a
                    deletion tombstone to the pack file or (2) in the large file
                    case remove the object from the file system.
                */
                panic!("no good, {:?}", id);
            }
            return Some(entry);
        }
        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::SeekFrom;
    use crate::util::*;

    #[test]
    fn test_store() {
        let tmp = TempDir::new().unwrap();
        let mut pb = tmp.path().to_path_buf();
        pb.push("example.btdb");
        let mut store = Store::new(pb);

        let id = random_object_id();
        assert_eq!(store.get_object(&id, true), None);
    }

    #[test]
    fn test_get() {
        let tmp = TempDir::new().unwrap();
        let mut pb = tmp.path().to_path_buf();
        pb.push("example.btdb");
        let store = Store::new(pb);
        let id = random_object_id();
        assert_eq!(store.get(&id), None);
        let mut guard = store.index.lock().unwrap();
        let entry = Entry {size: 3, offset: 5};
        assert_eq!(guard.insert(id.clone(), entry.clone()), None);
        // Release mutex lock otherwise following will deadlock:
        Mutex::unlock(guard);
        assert_eq!(store.get(&id), Some(entry));
    }

}

