//! Object hashing protocol.


use blake3;


pub trait Hasher {
    fn new() -> Self;
    fn hash_into(&self, info: u32, data: &[u8], hash: &mut [u8]);
}

pub struct Blake3 {

}

impl Hasher for Blake3 {
    fn new() -> Self {
        Self {}
    }

    fn hash_into(&self, info: u32, data: &[u8], hash: &mut [u8]) {
        assert!(hash.len() > 0 && hash.len() % 5 == 0);
        let mut h = blake3::Hasher::new();
        h.update(&info.to_le_bytes());
        h.update(data);
        h.finalize_xof().fill(hash);
    }
}

pub type DefaultHasher = Blake3;


#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use super::*;
    use crate::util::getrandom;

}

