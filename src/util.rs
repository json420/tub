//! Misc libc wrappers, currently just `getrandom()`.

use libc;


/// Make getrandom() syscall via libc.
pub fn getrandom(buf: &mut [u8]) {
    let size1 = buf.len();
    let p = buf.as_mut_ptr() as *mut libc::c_void;
    let size2 = unsafe {
        libc::getrandom(p, size1, 0)
    } as usize;
    if size1 != size2 {panic!("something went wrong")}
}


#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use super::*;

    #[test]
    fn test_getrandom() {
        let mut buf = [0_u8; 30];
        getrandom(&mut buf);
        assert_ne!(buf, [0; 30]);
        let og = buf.clone();
        getrandom(&mut buf);
        assert_ne!(buf, og);

        let mut set: HashSet<[u8; 30]> = HashSet::new();
        for _ in 0..6942 {
            getrandom(&mut buf);
            set.insert(buf.clone());
        }
        assert_eq!(set.len(), 6942);
    }
}

