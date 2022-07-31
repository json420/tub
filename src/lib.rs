use std::fs::File;
use std::io::prelude::*;

use blake2::{Blake2b, Digest};
use digest::consts::{U30,U10};
use generic_array::GenericArray;
use libc;


type Blake2b80 = Blake2b<U10>;
type Blake2b240 = Blake2b<U30>;


fn hash80(buf: &[u8]) -> GenericArray<u8, U10> {
    let mut h = Blake2b80::new();
    h.update(buf);
    h.finalize()
}

fn hash240(buf: &[u8]) -> GenericArray<u8, U30> {
    let mut h = Blake2b240::new();
    h.update(buf);
    h.finalize()
}

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

struct Store {
    file: File,
}

impl Store {
    fn write_object(&mut self, obj: &Object) -> std::io::Result<()> {
        // FIXME: Use write_all_vectored()
        self.file.write_all(&obj.hash)?;
        let size = obj.data.len() as u64;
        self.file.write_all(&size.to_le_bytes())?;
        self.file.write_all(&obj.data);
        Ok(())
    }
}




#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use super::*;

    static d1: &[u8] = b"my_input";
    static d1h80: [u8; 10] = hex!("2cc55c84e416924e6400");
    static d1h240: [u8; 30] = hex!("35f6b8fe184790c47717de56324629309370b1f37b1be1736027d414c122");

    #[test]
    fn test_hash80() {
        let mut h = Blake2b80::new();
        h.update(d1);
        let res = h.finalize();
        assert_eq!(res[..], (d1h80[..])[..]);

        let res = hash80(d1);
        assert_eq!(res[..], d1h80[..]);
    }

    #[test]
    fn test_hash240() {
        let mut h = Blake2b240::new();
        h.update(d1);
        let res = h.finalize();
        assert_eq!(res[..], (d1h240[..])[..]);

        let res = hash240(d1);
        assert_eq!(res[..], d1h240[..]);
    }

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
    fn test_store() {
    
    }
}
