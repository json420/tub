//! Object hashing protocol.



// https://bazaar.launchpad.net/~dmedia/filestore/trunk/view/head:/filestore/protocols.py

/*  FIXME: Skein probably provides better performance and a better security
    margin than Blake2b, so we should strongly consider Skein.
*/
use std::fmt;
use blake3;
use crate::base::*;
use crate::dbase32::db32enc_str;


#[derive(Debug, PartialEq)]
pub struct LeafInfo {
    pub hash: TubHash,
    pub index: u64,
}

impl LeafInfo {
    pub fn new(hash: TubHash, index: u64) -> Self {
        Self {hash: hash, index: index}   
    }

    pub fn as_db32(&self) -> String {
        db32enc_str(&self.hash)
    }
}


pub fn hash_leaf(index: u64, data: &[u8]) -> TubHash {
    assert!(data.len() > 0);
    let mut h = blake3::Hasher::new();
    h.update(b"Tub/leaf_hash");  // <-- FIXME: Do more better than this
    h.update(&index.to_le_bytes());
    h.update(data);
    let mut hash: TubHash = [0_u8; TUB_HASH_LEN];
    h.finalize_xof().fill(&mut hash);
    hash
}

pub fn hash_root(size: u64, leaf_hashes: &[u8]) -> TubHash {
    assert!(leaf_hashes.len() > 0);
    assert!(leaf_hashes.len() % TUB_HASH_LEN == 0);
    let mut h = blake3::Hasher::new();
    //h.update(b"Tub/root_hash");  // <-- FIXME: Do more better than this
    h.update(&size.to_le_bytes());
    h.update(leaf_hashes);
    let mut hash: TubHash = [0_u8; TUB_HASH_LEN];
    h.finalize_xof().fill(&mut hash);
    hash
}

pub fn hash_tombstone(hash: &TubHash) -> TubHash {
    let mut h = blake3::Hasher::new();
    h.update(b"Tub/tombstone_hash");  // <-- FIXME: Do more better than this
    h.update(hash);
    h.update(&0_u64.to_le_bytes());
    let mut marker: TubHash = [0_u8; TUB_HASH_LEN];
    h.finalize_xof().fill(&mut marker);
    marker
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::{random_small_object, random_hash};
    use crate::dbase32::db32enc_str;

    #[test]
    fn test_hash_leaf() {

    }

    #[test]
    fn test_hash_root() {

    }

    #[test]
    fn test_hash_tombstone() {

    }
}

