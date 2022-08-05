mod protocol;

use std::fs::File;
use std::io::prelude::*;

use libc;


fn get_random(buf: &mut [u8]) {
    let size1 = buf.len();
    let p = buf.as_mut_ptr() as *mut libc::c_void;
    let size2 = unsafe {
        libc::getrandom(p, size1, 0)
    } as usize;
    if size1 != size2 {panic!("something went wrong")}
}



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
        self.file.write_all(&size.to_le_bytes())?;
        self.file.write_all(&obj.data);
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
        
        Some(Object{hash: hash, data})
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_random() {
        let b1 = &mut [0_u8; 30];
        assert_eq!(b1[..], [0_u8; 30][..]);
        get_random(b1);
        assert_ne!(b1[..], [0_u8; 30][..]);
        let b2 = &mut [0_u8, 30];
        get_random(b2);
        assert_ne!(b1[..], b2[..]);
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
