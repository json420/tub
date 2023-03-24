//! Object hashing protocol.

use std::marker::PhantomData;
use std::io::Result as IoResult;

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
        assert!(! hash.is_empty() && hash.len() % 5 == 0);
        let mut h = blake3::Hasher::new();
        if payload.len() > 131072 {
            h.update_rayon(payload);
        }
        else {
            h.update(payload);
        }
        h.finalize_xof().fill(hash);
    }
}

pub type DefaultHasher = Blake3;


pub trait Protocol {
    fn digest() -> usize {
        30
    }

    fn size() -> usize {
        3
    }

    fn header() -> usize {
        Self::digest() + Self::size() + 1
    }

}

pub struct Hash<const N: usize> {
    buf: [u8; N],
}

pub struct Object<P: Protocol> {
    phantom: PhantomData<P>,
    buf: Vec<u8>,
}

impl<P: Protocol> Object<P> {
    fn reset(&mut self) {
        self.buf.clear();
        self.buf.resize(P::header(), 0);
    }
}

pub trait Store<P: Protocol> {
    fn save(&self, obj: &Object<P>) -> IoResult<bool>;
}


#[cfg(test)]
mod tests {
    use super::*;
    use getrandom::getrandom;
    use crate::helpers::flip_bit_in;
    use std::collections::HashSet;

    #[test]
    fn test_blake3() {
        let mut hash = [0_u8; 30];
        let mut data = [0_u8; 69];
        getrandom(&mut data).unwrap();
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

