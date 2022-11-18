//! Leaf-wise File IO.
//!
//! In general, anything that uses LEAF_SIZE should be here.

use crate::base::LEAF_SIZE;


pub fn new_leaf_buf() -> Vec<u8> {
    let mut buf = Vec::with_capacity(LEAF_SIZE as usize);
    buf.resize(LEAF_SIZE as usize, 0);
    buf
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_leaf_buf() {
        let mut buf = new_leaf_buf();
        assert_eq!(buf.len(), LEAF_SIZE as usize);
        assert_eq!(buf.capacity(), LEAF_SIZE as usize);
        //let s = &mut buf[0..111];
    }
}
