//! Object hashing protocol.



// https://bazaar.launchpad.net/~dmedia/filestore/trunk/view/head:/filestore/protocols.py

/*  FIXME: Skein probably provides better performance and a better security
    margin than Blake2b, so we should strongly consider Skein.
*/
use blake3;
use crate::base::*;


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

pub fn hash_with_pers(data: &[u8], pers: &[u8]) -> TubHash {
    assert!(data.len() > 0);
    let mut h = blake3::Hasher::new();
    h.update(pers);  // <-- FIXME: Do more better than this
    h.update(data);
    let mut hash: TubHash = [0_u8; TUB_HASH_LEN];
    h.finalize_xof().fill(&mut hash);
    hash
}

pub fn hash_small_object(data: &[u8]) -> TubHash {
    hash_with_pers(data, b"Tub/small_object")
}

pub fn hash_leaf_hashes(data: &[u8]) -> TubHash {
    assert!(data.len() >= TUB_HASH_LEN * 2);
    assert!(data.len() % TUB_HASH_LEN == 0);
    hash_with_pers(data, b"Tub/leaf_hash_list")
}

pub fn hash_payload(size: u64, data: &[u8]) -> TubHash {
    if size > LEAF_SIZE {
        hash_leaf_hashes(data)
    }
    else {
        hash_small_object(data)
    }
}

pub fn hash_root(size: u64, payload_hash: &TubHash) -> TubHash {
    assert!(size > 0);
    let mut h = blake3::Hasher::new();
    h.update(b"Tub/root_hash");  // <-- FIXME: Do more better than this
    h.update(&size.to_le_bytes());
    h.update(payload_hash);
    let mut hash: TubHash = [0_u8; TUB_HASH_LEN];
    h.finalize_xof().fill(&mut hash);
    hash
}

pub fn hash_root2(tail: &[u8]) -> TubHash {
    let kind: ObjectType = tail[8].into();
    assert!(tail.len() == TAIL_LEN);
    let mut h = blake3::Hasher::new();
    h.update(tail);
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
    use std::collections::HashSet;
    use super::*;
    use crate::util::{random_object, random_hash};

    const COUNT: usize = 1000;

    #[test]
    fn test_hash_leaf() {
        for size in [1, 2, 42, 420] {
            let data = random_object(size);

            // Should be tied to index
            let mut set: HashSet<TubHash> = HashSet::new();
            for i in 0..COUNT {
                set.insert(hash_leaf(i as u64, &data));
            }
            assert_eq!(set.len(), COUNT);

            // Should be tied every byte in data
            let mut set: HashSet<TubHash> = HashSet::new();
            let hash = hash_leaf(0, &data);
            for i in 0..data.len() {
                for v in 0_u8..=255 {
                    let mut copy = data.clone();
                    copy[i] = v;
                    let newhash = hash_leaf(0, &copy);
                    set.insert(newhash);
                    if data[i] == copy[i] {
                        assert_eq!(hash, newhash);
                    }
                    else {
                        assert_ne!(hash, newhash);
                    }
                }
            }
            assert_eq!(set.len(), data.len() * 255 + 1);
        }
    }

    #[test]
    fn test_hash_small_object() {
        for size in [1, 2, 42, 69, 420] {
            let data = random_object(size);

            // Should be tied every byte in data
            let mut set: HashSet<TubHash> = HashSet::new();
            let hash = hash_small_object(&data);
            for i in 0..data.len() {
                for v in 0_u8..=255 {
                    let mut copy = data.clone();
                    copy[i] = v;
                    let newhash = hash_small_object(&copy);
                    set.insert(newhash);
                    if data[i] == copy[i] {
                        assert_eq!(hash, newhash);
                    }
                    else {
                        assert_ne!(hash, newhash);
                    }
                }
            }
            assert_eq!(set.len(), data.len() * 255 + 1);
        }
    }

    #[test]
    fn test_hash_root() {
        let payload_hash = random_hash();

        // Should be tied to size
        let mut set: HashSet<TubHash> = HashSet::new();
        for size in 1..COUNT + 1 {
            set.insert(hash_root(size as u64, &payload_hash));
        }
        assert_eq!(set.len(), COUNT);

        // Should be tied to every byte in payload_hash
        let mut set: HashSet<TubHash> = HashSet::new();
        for i in 0..payload_hash.len() {
            for v in 0_u8..=255 {
                let mut copy = payload_hash.clone();
                copy[i] = v;
                set.insert(hash_root(1, &copy));
            }
        }
        assert_eq!(set.len(), payload_hash.len() * 255 + 1);
    }

    #[test]
    fn test_hash_tombstone() {
        let hash = random_hash();

        // Should be tied every byte in hash
        let mut set: HashSet<TubHash> = HashSet::new();
        for i in 0..hash.len() {
            for v in 0_u8..=255 {
                let mut copy = hash.clone();
                copy[i] = v;
                let tombstone = hash_tombstone(&copy);
                set.insert(tombstone);
            }
        }
        assert_eq!(set.len(), hash.len() * 255 + 1);
    }
}

