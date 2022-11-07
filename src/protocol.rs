//! Object hashing protocol.


/*  FIXME: Skein probably provides better performance and a better security
    margin than Blake2b, so we should strongly consider Skein.
*/
use blake3;
use crate::base::*;

pub type HashFunc = blake3::Hasher;


pub fn hash(buf: &[u8]) -> ObjectID {
    let mut h = blake3::Hasher::new();
    h.update(buf);
    let mut out = h.finalize_xof();
    let mut id: ObjectID = [0_u8; OBJECT_ID_LEN];
    out.fill(&mut id);
    id
    
}


// Abstracts away the details of specific hash fuctions, etc.
trait Protocol {
    fn new() -> Self;
    fn hash_leaf(&self, index: LeafIndex, data: &[u8]) -> LeafHash;
    fn hash_root(&self, size: ObjectSize, leaves: LeafHashList) -> ObjectID;
}

struct Blake3Protocol {

}

impl Protocol for Blake3Protocol {
    fn new() -> Self {
        Self {}
    }

    fn hash_leaf(&self, index: LeafIndex, data: &[u8]) -> LeafHash {
        let mut h = blake3::Hasher::new();
        h.update(&index.to_le_bytes());
        h.update(data);
        let mut lh: LeafHash = [0_u8; LEAF_HASH_LEN];
        h.finalize_xof().fill(&mut lh);
        lh
    }

    fn hash_root(&self, size: ObjectSize, leaf_hashes: LeafHashList) -> ObjectID {
        let mut h = blake3::Hasher::new();
        h.update(&size.to_le_bytes());
        for leaf_hash in leaf_hashes.iter() {
            h.update(leaf_hash);
        }
        let mut id: ObjectID = [0_u8; OBJECT_ID_LEN];
        h.finalize_xof().fill(&mut id);
        id
    }
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
        h.update(&index.to_le_bytes());
        h.update(data);
        let mut id: ObjectID = [0_u8; OBJECT_ID_LEN];
        h.finalize_xof().fill(&mut id);
        self.leaf_hashes.push(id);
        self.size += data.len() as ObjectSize;
        if data.len() < LEAF_SIZE as usize {
            self.closed = true;
        }
    }

    fn content_hash(&mut self) -> ObjectID {
        self.closed = true;
        let mut h = HashFunc::new();
        h.update(&self.size.to_le_bytes());
        for leaf_hash in self.leaf_hashes.iter() {
            h.update(leaf_hash);
        }
        let mut id: ObjectID = [0_u8; OBJECT_ID_LEN];
        h.finalize_xof().fill(&mut id);
        id
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbase32::db32enc_str;

    #[test]
    fn test_hash() {
        let id = hash(b"Federation44");
        assert_eq!(&db32enc_str(&id),
            "OK5UTJXH6H3Q9DU7EHY9LEAN8P6TPY553SIGLQH5KAXEG6EN"
        );
    }

    #[test]
    fn test_hasher() {
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

