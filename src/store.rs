use std::fs::File;
use std::io::prelude::*;


#[derive(Debug, PartialEq)]
struct Object {
    hash: [u8; 30],
    data: Vec<u8>,
}

impl Object {
    fn size(&self) -> u64 {
        self.data.len() as u64
    }
}

struct Store {
    file: File,
}

impl Store {
    fn write_object(&mut self, obj: &Object) -> std::io::Result<()> {
        // FIXME: Use write_all_vectored()
        self.file.write_all(&obj.hash)?;
        let size: u64 = obj.data.len() as u64;
        self.file.write_all(&obj.size().to_le_bytes())?;
        self.file.write_all(&obj.data);
        self.file.flush()?;
        Ok(())
    }

    fn read_next_object(&mut self) -> Option<Object> {
        let mut header = [0_u8; 38];
        if let Err(_) = self.file.read_exact(&mut header) {
            return None;
        }
        let size_buf: [u8; 8] = header[0..8].try_into().expect("no good");
        let size = u64::from_le_bytes(size_buf);
        let hash: [u8; 30] = header[8..40].try_into().expect("no good");
        let mut data: Vec<u8> = Vec::with_capacity(size as usize);
        self.file.read_exact(&mut data);
        Some(Object{hash: hash, data})
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::SeekFrom;

    #[test]
    fn test_store() {
        let tmp = TempDir::new().unwrap();
        let mut pb = tmp.path().to_path_buf();
        pb.push("example.btdb");
        let mut store = Store{file: File::create(pb).unwrap()};
        assert_eq!(store.read_next_object(), None);
    }

    #[test]
    fn test_write() {
        let tmp = TempDir::new().unwrap();
        let mut pb = tmp.path().to_path_buf();
        pb.push("example.btdb");
        let mut store = Store{file: File::create(pb).unwrap()};
        assert_eq!(store.read_next_object(), None);
        let obj = Object{hash: [0_u8; 30], data: vec![1_u8; 420]};
        assert!(store.write_object(&obj).is_ok());
        assert_eq!(store.file.stream_position().unwrap(), 458);

        store.file.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(store.file.stream_position().unwrap(), 0);
        assert_eq!(store.file.stream_len().unwrap(), 458);
        //assert_eq!(store.read_next_object(), Some(obj));
    }



    #[test]
    fn test_read_next_object() {
        let tmp = TempDir::new().unwrap();
        let mut pb = tmp.path().to_path_buf();
    }

    #[test]
    fn test_object() {
        let o = Object {
            hash: [0_u8; 30],
            data: vec![],
        };
        assert_eq!(o.size(), 0);
    }
}

