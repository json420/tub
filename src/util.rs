use libc;
use crate::base::*;


fn getrandom(buf: &mut [u8]) {
    let size1 = buf.len();
    let p = buf.as_mut_ptr() as *mut libc::c_void;
    let size2 = unsafe {
        libc::getrandom(p, size1, 0)
    } as usize;
    if size1 != size2 {panic!("something went wrong")}
}


fn random_id() -> AbstractID {
    let mut id = [0_u8; ABSTRACT_ID_SIZE];
    getrandom(&mut id);
    id
}


fn random_object_id() -> ObjectID {
    let mut id = [0_u8; OBJECT_ID_SIZE];
    getrandom(&mut id);
    id
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_getrandom() {
        let b1 = &mut [0_u8; 30];
        assert_eq!(b1[..], [0_u8; 30][..]);
        getrandom(b1);
        assert_ne!(b1[..], [0_u8; 30][..]);
        let b2 = &mut [0_u8; 30];
        getrandom(b2);
        assert_ne!(b1[..], b2[..]);

        let b3 = &mut [0_u8; 65536];
        assert_eq!(b3[..], [0_u8; 65536][..]);
        getrandom(b3);
        assert_ne!(b3[..], [0_u8; 65536][..]);
    }

    #[test]
    fn test_random_id() {
        let a = random_id();
        let b = random_id();
        assert_ne!(a, b);        
    }

    #[test]
    fn test_random_object_id() {
        let a = random_object_id();
        let b = random_object_id();
        assert_ne!(a, b);        
    }
}
