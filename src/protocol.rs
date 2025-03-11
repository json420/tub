//! Object hashing protocol.

use std::io::Result as IoResult;
use std::marker::PhantomData;

use blake3;

pub trait Hasher {
    fn new() -> Self;
    fn hash_into(&self, data: &[u8], hash: &mut [u8]);
}

pub struct Blake3 {}

impl Hasher for Blake3 {
    fn new() -> Self {
        Self {}
    }

    fn hash_into(&self, payload: &[u8], hash: &mut [u8]) {
        assert!(!hash.is_empty() && hash.len() % 5 == 0);
        let mut h = blake3::Hasher::new();
        if payload.len() > 131072 {
            h.update_rayon(payload);
        } else {
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

pub struct HashIter<const N: usize> {}

impl<const N: usize> Iterator for HashIter<N> {
    type Item = Hash<N>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub struct Object<P: Protocol, const N: usize> {
    phantom: PhantomData<P>,
    buf: Vec<u8>,
}

impl<P: Protocol, const N: usize> Object<P, N> {
    fn reset(&mut self) {
        self.buf.clear();
        self.buf.resize(P::header(), 0);
    }

    pub fn as_header(&self) -> &[u8] {
        &self.buf[0..P::header()]
    }
}

pub trait Store<P: Protocol, const N: usize> {
    fn save(&mut self, obj: &Object<P, N>) -> IoResult<bool>;

    fn load(&mut self, obj: Object<P, N>, hash: &Hash<N>) -> IoResult<bool>;

    fn delete(&mut self, hash: &Hash<N>) -> IoResult<bool>;

    //fn iter(&self) -> HashIter2<P, N, Store<P, N>>;
}

pub struct HashIter2<P: Protocol, const N: usize, S: Store<P, N>> {
    store: S,
    phantom1: PhantomData<P>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::flip_bit_in;
    use getrandom;
    use std::collections::HashSet;

    #[test]
    fn test_blake3() {
        let mut hash = [0_u8; 30];
        let mut data = [0_u8; 69];
        getrandom::fill(&mut data).unwrap();
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
            flip_bit_in(&mut data, bit); // Flip bit back
            b3.hash_into(&data, &mut hash);
            assert_eq!(hash, og);
        }
        assert_eq!(set.len(), 69 * 8 + 1);
    }
}
