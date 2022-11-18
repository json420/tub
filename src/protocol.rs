//! Object hashing protocol.



// https://bazaar.launchpad.net/~dmedia/filestore/trunk/view/head:/filestore/protocols.py

/*  FIXME: Skein probably provides better performance and a better security
    margin than Blake2b, so we should strongly consider Skein.
*/
use blake3;
use crate::base::*;


pub fn hash(buf: &[u8]) -> ObjectID {
    let mut h = blake3::Hasher::new();
    h.update(buf);
    let mut out = h.finalize_xof();
    let mut id: ObjectID = [0_u8; OBJECT_ID_LEN];
    out.fill(&mut id);
    id
    
}


pub fn hash_leaf(index: LeafIndex, data: &[u8]) -> LeafHash {
    let mut h = blake3::Hasher::new();
    h.update(&index.to_le_bytes());
    h.update(data);
    let mut lh: LeafHash = [0_u8; LEAF_HASH_LEN];
    h.finalize_xof().fill(&mut lh);
    lh
}


pub fn hash_root(size: ObjectSize, leaf_hashes: LeafHashList) -> ObjectID {
    let mut h = blake3::Hasher::new();
    h.update(&size.to_le_bytes());
    for leaf_hash in leaf_hashes.iter() {
        h.update(leaf_hash);
    }
    let mut id: ObjectID = [0_u8; OBJECT_ID_LEN];
    h.finalize_xof().fill(&mut id);
    id
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
}
