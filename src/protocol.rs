//! Object hashing protocol.



// https://bazaar.launchpad.net/~dmedia/filestore/trunk/view/head:/filestore/protocols.py

/*  FIXME: Skein probably provides better performance and a better security
    margin than Blake2b, so we should strongly consider Skein.
*/
use blake3;
use crate::base::*;
use crate::dbase32::db32enc_str;


pub fn hash(buf: &[u8]) -> TubHash {
    assert!(buf.len() as u64 <= LEAF_SIZE);
    let leaf = hash_leaf(0, buf);
    hash_root(buf.len() as u64, vec![leaf.hash]).hash
}


#[derive(Debug, PartialEq)]
pub struct LeafInfo {
    pub hash: TubHash,
    pub index: u64,
}

impl LeafInfo {
    pub fn as_db32(&self) -> String {
        db32enc_str(&self.hash)
    }
}

pub fn hash_leaf(index: u64, data: &[u8]) -> LeafInfo {
    let mut h = blake3::Hasher::new();
    h.update(&index.to_le_bytes());
    h.update(data);
    let mut hash: TubHash = [0_u8; TUB_HASH_LEN];
    h.finalize_xof().fill(&mut hash);
    LeafInfo {index: index, hash: hash}
}


#[derive(Debug, PartialEq)]
pub struct RootInfo {
    pub hash: TubHash,
    pub size: u64,
    pub leaf_hashes: TubHashList,
}

impl RootInfo {
    pub fn as_db32(&self) -> String {
        db32enc_str(&self.hash)
    }
}

pub fn hash_root(size: u64, leaf_hashes: TubHashList) -> RootInfo {
    let mut h = blake3::Hasher::new();
    h.update(&size.to_le_bytes());
    for leaf_hash in leaf_hashes.iter() {
        h.update(leaf_hash);
    }
    let mut id: TubHash = [0_u8; TUB_HASH_LEN];
    h.finalize_xof().fill(&mut id);
    RootInfo {size: size, hash: id, leaf_hashes: leaf_hashes}
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::{random_small_object, random_hash};
    use crate::dbase32::db32enc_str;

    #[test]
    fn test_hash() {
        let id = hash(b"Federation44");
        assert_eq!(&db32enc_str(&id),
            "TDJGJI47CFS53WQWE7K77R8GJVIAE9KB6465SPUV6NDYPVKA"
        );
    }

    #[test]
    fn test_hashe_leaf() {
        let obj = random_small_object();
        let lh0 = hash_leaf(0, &obj);
        let lh1 = hash_leaf(1, &obj);
        assert_eq!(lh0.index, 0);
        assert_eq!(lh1.index, 1);
        assert_ne!(lh0.hash, lh1.hash);
    }

    #[test]
    fn test_hash_root() {
        let lh0 = random_hash();
        let lh1 = random_hash();
        let a = hash_root(1, vec![lh0]);
        let b = hash_root(LEAF_SIZE, vec![lh0]);
        let c = hash_root(LEAF_SIZE + 1, vec![lh0, lh1]);
        let d = hash_root(2 * LEAF_SIZE, vec![lh0, lh1]);
        assert_eq!(a.leaf_hashes, b.leaf_hashes);
        assert_ne!(a.hash, b.hash);
        assert_eq!(c.leaf_hashes, d.leaf_hashes);
        assert_ne!(c.hash, d.hash);
    }
}
