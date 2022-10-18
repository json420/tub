use libc;


fn getrandom(buf: &mut [u8]) {
    let size1 = buf.len();
    let p = buf.as_mut_ptr() as *mut libc::c_void;
    let size2 = unsafe {
        libc::getrandom(p, size1, 0)
    } as usize;
    if size1 != size2 {panic!("something went wrong")}
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
}
