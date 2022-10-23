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



#[derive(Debug, PartialEq, Clone, Copy)]
struct Entry {
    offset: OffsetSize,
    size: ObjectSize,
}

type Index = Arc<Mutex<HashMap<ObjectID, Entry>>>;


#[derive(Debug)]
pub struct Store {
    file: File,
    index: Index,
}


impl Store {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let index: Index = Arc::new(Mutex::new(HashMap::new()));
        let file = File::options()
            .append(true)
            .read(true)
            .create(true)
            .open(path)
            .expect("could not open pack file");
        Store {
            file: file,
            index: index,
        }
    }

    /*
    fn read_next_object(&mut self) -> Option<Object> {
        let mut header = [0_u8; 38];
        if let Err(_) = self.file.read_exact(&mut header) {
            return None;
        }
        let size_buf: [u8; 8] = header[0..8].try_into().expect("no good");
        let size = u64::from_le_bytes(size_buf);
        let hash: [u8; 30] = header[8..40].try_into().expect("no good");
        let mut data: Vec<u8> = Vec::with_capacity(size as usize);
        self.file.read_exact(&mut data).unwrap();
        Some(Object{hash: hash, data})
    }
    */

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
            println!("{}\t{}", offset, size);

            let entry = Entry {
                offset: offset,
                size: size,
            };
            assert_eq!(index.insert(id, entry), None);

            offset += HEADER_LEN as ObjectSize + size;

            if check {
                buf.resize(size as usize, 0);
                let s = &mut buf[0..(size as usize)];
                self.file.read_exact(s).expect("oops");
                hash(s);
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

    fn set(&self, id: ObjectID, entry: Entry) -> Option<Entry> {
        let mut index = self.index.lock().unwrap();
        index.insert(id, entry)
    }

    fn add_object(&mut self, data: &[u8]) -> (ObjectID, Entry) {
        let id = hash(data);
        let mut index = self.index.lock().unwrap();
        if let Some(entry) = index.get(&id) {
            return (id, entry.clone());  // Already in object store
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
        (id, entry)
    }

    fn get_object(&mut self, id: &ObjectID) -> Option<Entry> {
        if let Some(entry) = self.get(id) {
            let mut buf = vec![0_u8; entry.size as usize];
            assert_eq!(buf.len(), entry.size as usize);
            self.file.read_exact_at(
                &mut buf[0..entry.size as usize],
                entry.offset
            ).expect("oops");
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
    use hex_literal::hex;

    static D1: &[u8] = b"my_input";
    static D1H240: [u8; 30] = hex!("35f6b8fe184790c47717de56324629309370b1f37b1be1736027d414c122");

    #[test]
    fn test_store() {
        let tmp = TempDir::new().unwrap();
        let mut pb = tmp.path().to_path_buf();
        pb.push("example.btdb");
        let mut store = Store::new(pb);

        let id = random_object_id();
        assert_eq!(store.get_object(&id), None);
        /*
        assert_eq!(store.add_object(D1),
            (D1H240, Entry {offset: 0, size: 8})
        );
        assert_eq!(store.get(&D1H240), Some(Entry {offset: 0, size: 8}));
        assert_eq!(store.get_object(&D1H240), Some(Entry {offset: 0, size: 8}));
        */
    }

    #[test]
    fn test_get() {
        let tmp = TempDir::new().unwrap();
        let mut pb = tmp.path().to_path_buf();
        pb.push("example.btdb");
        let mut store = Store::new(pb);
        let id = random_object_id();
        assert_eq!(store.get(&id), None);
        let mut guard = store.index.lock().unwrap();
        let entry = Entry {size: 3, offset: 5};
        assert_eq!(guard.insert(id.clone(), entry.clone()), None);
        // Release mutex lock otherwise following will deadlock:
        Mutex::unlock(guard);
        assert_eq!(store.get(&id), Some(entry));
    }

    #[test]
    fn test_set() {
        let tmp = TempDir::new().unwrap();
        let mut pb = tmp.path().to_path_buf();
        pb.push("example.btdb");
        let mut store = Store::new(pb);
        let id = random_object_id();
        let entry = Entry {size: 3, offset: 5};
        assert_eq!(store.set(id.clone(), entry.clone()), None);
        let entry2 = Entry {size: 7, offset: 11};
        assert_eq!(store.set(id.clone(), entry2.clone()), Some(entry));
    }
}

