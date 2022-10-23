/*  FIXME: Skein probably provides better performance and a better security
    margin than Blake2b, so we should strongly consider Skein.
*/
use std::cmp::min;
use blake2::{Blake2b, Digest};
use digest::consts::{U30};
use generic_array::GenericArray;
use blake3;
use crate::base::*;

type HashFunc = Blake2b<U30>;


pub fn hash(buf: &[u8]) -> ObjectID {
    let mut h = HashFunc::new();
    h.update(buf);
    ObjectID::from(h.finalize())
}

pub fn hash2(buf: &[u8]) -> blake3::Hash {
    blake3::hash(buf)
}


struct Hasher {
    closed: bool,
    size: ObjectSize,
    leaf_hashes: LeafHashList,
}


impl Hasher {
    fn new() -> Self {
        Self {
            closed: false,
            size: 0,
            leaf_hashes: vec![],
        }
    }

    fn hash_leaf(&mut self, index: usize, data: &[u8]) {
        assert!(!self.closed);
        assert_eq!(index as ObjectSize, self.size / LEAF_SIZE);
        assert!(data.len() > 0);
        assert!(data.len() <= LEAF_SIZE as usize);
        let mut h = HashFunc::new();
        h.update(index.to_le_bytes());
        h.update(data);
        self.leaf_hashes.push(LeafHash::from(h.finalize()));
        self.size += data.len() as ObjectSize;
        if data.len() < LEAF_SIZE as usize {
            self.closed = true;
        }
    }

    fn content_hash(&mut self) -> ObjectID {
        self.closed = true;
        let mut h = HashFunc::new();
        h.update(self.size.to_le_bytes());
        for leaf_hash in self.leaf_hashes.iter() {
            h.update(leaf_hash);
        }
        ObjectID::from(h.finalize())
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
    fn test_Hasher() {
        let mut h: Hasher = Hasher::new();
        assert_eq!(h.closed, false);
        assert_eq!(h.size, 0);
        assert_eq!(h.leaf_hashes.len(),  0);

        h.hash_leaf(0, b"my_input");
        assert_eq!(h.closed, true);
        assert_eq!(h.size, 8);
        assert_eq!(h.leaf_hashes.len(),  1);

        let mut h: Hasher = Hasher::new();
        h.hash_leaf(0, &vec![44_u8; LEAF_SIZE as usize]);
        assert_eq!(h.closed, false);
        assert_eq!(h.size, LEAF_SIZE);
        assert_eq!(h.leaf_hashes.len(),  1);

        h.hash_leaf(1, b"my_input");
        assert_eq!(h.closed, true);
        assert_eq!(h.size, LEAF_SIZE + 8);
        assert_eq!(h.leaf_hashes.len(),  2);
        

    }

}

