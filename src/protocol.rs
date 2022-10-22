/*  FIXME: Skein probably provides better performance and a better security
    margin than Blake2b, so we should strongly consider Skein.
*/
use std::cmp::min;
use blake2::{Blake2b, Digest};
use digest::consts::{U30};
use generic_array::GenericArray;
use crate::base::*;

type HashFunc = Blake2b<U30>;


pub fn hash(buf: &[u8]) -> GenericArray<u8, U30> {
    let mut h = HashFunc::new();
    h.update(buf);
    h.finalize()
}



struct LeafHasher {
    h: HashFunc,
    size: usize,
}


impl LeafHasher {
    fn new() -> Self {
        Self {
            h: HashFunc::new(),
            size: 0,
        }
    }

    fn remaining(&self) -> usize {
        LEAF_SIZE - self.size
    }

    fn update(&mut self, data: &Vec<u8>) {
        self.size += data.len();
        assert!(self.size <= LEAF_SIZE);
        self.h.update(data);
    }

    fn finalize_reset(&mut self) -> (LeafHash, usize) {
        assert!(self.size > 0);
        assert!(self.size <= LEAF_SIZE);
        (LeafHash::from(self.h.finalize_reset()), self.size)
    }

    fn finalize(self) -> (LeafHash, usize) {
        assert!(self.size > 0);
        assert!(self.size <= LEAF_SIZE);
        (LeafHash::from(self.h.finalize()), self.size)
    }
}


struct Hasher {
    leaf_hasher: LeafHasher,
    size: ObjectSize,
    leaf_hashes: LeafHashList,
}


impl Hasher {
    fn new() -> Self {
        Self {
            leaf_hasher: LeafHasher::new(),
            size: 0,
            leaf_hashes: vec![],
        }
    }

    fn next_leaf(&mut self) {
        //self.leaf_hasher = LeafHasher::new();
        //let (leaf_hash, size) = self.leaf_hasher.finalize();
    }

    fn update(&mut self, data: &Vec<u8>) {
        let mut cur: usize = 0;
        while cur < data.len() {
            let s = min(data.len() - cur, self.leaf_hasher.remaining());
            let myslice = &data[cur..cur+s];
            cur += s;
        }
        self.size += data.len() as ObjectSize;
    }


    fn hash_leaf(&mut self, data: &[u8]) {
        self.leaf_hashes.push(LeafHash::from(hash(data)));
        self.size += data.len() as ObjectSize;
    }

}


#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use super::*;

    static D1: &[u8] = b"my_input";
    static D1H240: [u8; 30] = hex!("35f6b8fe184790c47717de56324629309370b1f37b1be1736027d414c122");

    #[test]
    fn test_hash() {
        let mut h = HashFunc::new();
        h.update(D1);
        let res = h.finalize();
        assert_eq!(res[..], (D1H240[..])[..]);

        let res = hash(D1);
        assert_eq!(res[..], D1H240[..]);
    }

    #[test]
    fn test_LeafHasher() {
        let mut lh = LeafHasher::new();
        assert_eq!(lh.size, 0);
        assert_eq!(lh.remaining(), LEAF_SIZE);
    }

    #[test]
    fn test_Hasher_new() {
        let h: Hasher = Hasher::new();
        assert_eq!(h.size, 0);
        assert_eq!(h.leaf_hashes.len(),  0);
    }

    #[test]
    fn test_Hasher_update() {
        let mut h = Hasher::new();
        let data = Box::new(vec![1_u8;  231]);
        h.update(&data);
        assert_eq!(h.size, 231);
        //assert_eq!(h.leaf_hashes.len(),  1);
    }

    #[test]
    fn test_Hasher_hash_leaf() {
        let mut h = Hasher::new();
        let data = [1_u8;  231];
        h.hash_leaf(&data);
        assert_eq!(h.size, 231);
        assert_eq!(h.leaf_hashes.len(),  1);
    }
}

