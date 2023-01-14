//! Object hashing protocol.


use blake3;


pub trait Hasher {
    fn new() -> Self;
    fn hash_into(&self, data: &[u8], hash: &mut [u8]);
}

pub struct Blake3 {

}

impl Hasher for Blake3 {
    fn new() -> Self {
        Self {}
    }

    fn hash_into(&self, payload: &[u8], hash: &mut [u8]) {
        //!(hash.len() > 0 && hash.len() % 5 == 0);
        let mut h = blake3::Hasher::new();
        h.update_rayon(payload);
        h.finalize_xof().fill(hash);
    }
}

pub type DefaultHasher = Blake3;


#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::getrandom;
    use crate::helpers::flip_bit_in;
    use std::collections::HashSet;

    #[test]
    fn test_blake3() {
        let mut hash = [0_u8; 30];
        let mut data = [0_u8; 69];
        getrandom(&mut data);
        let b3 = Blake3::new();
        b3.hash_into(&data, &mut hash);
        let mut set: HashSet<[u8; 30]> = HashSet::new();
        let og = hash.clone();
        set.insert(hash.clone());
        for bit in 0..data.len() * 8 {
            flip_bit_in(&mut data, bit);
            b3.hash_into(&data, &mut hash);
            assert_ne!(hash, og);
            assert!(set.insert(hash.clone()));
            flip_bit_in(&mut data, bit);  // Flip bit back
            b3.hash_into(&data, &mut hash);
            assert_eq!(hash, og);
        }
        assert_eq!(set.len(), 69 * 8 + 1);
    }
}

