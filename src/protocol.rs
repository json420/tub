/*  FIXME: Skein probably provides better performance and a better security
    margin than Blake2b, so we should strongly consider Skein.
*/
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


struct Hasher {
    hasher: HashFunc,
    size: ObjectSize,
    leaf_hashes: LeafHashList,
}


impl Hasher {
    fn new() -> Self {
        Hasher {
            hasher: HashFunc::new(),
            size: 0,
            leaf_hashes: vec![],
        }
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
    fn test_Hasher_new() {
        let h: Hasher = Hasher::new();
        assert_eq!(h.size, 0);
        assert_eq!(h.leaf_hashes.len(),  0);
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

