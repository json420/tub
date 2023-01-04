//! Object hashing protocol.

/*

Like Git, Tub does a lot of hashing, so the performance of the hash function we
use is critical.  There seem to be two high-performance contenders: 

    1. Blake3 - https://github.com/BLAKE3-team/BLAKE3

    2. Kangaroo Twelve - https://keccak.team/kangarootwelve.html

Large object protocol based on the Dmedia hashing protocol:

https://bazaar.launchpad.net/~dmedia/filestore/trunk/view/head:/filestore/protocols.py

*/
use std::ops;
use blake3;
use crate::base::*;
use crate::dbase32::db32enc;
use std::fmt;


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

pub fn hash_root(tail: &[u8]) -> TubHash {
    if tail.len() != TAIL_LEN {
        panic!("Need buffer {} bytes long, got {}", TAIL_LEN, tail.len());
    }
    let mut h = blake3::Hasher::new();
    h.update(b"Tub/root_hash");  // <-- FIXME: Do more better than thiss
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


// FIXME: Can we put compile time contraints on N such that N > 0 && N % 5 == 0?
#[derive(Debug, PartialEq, Eq)]
struct TubName<const N: usize> {
    pub buf: [u8; N],
}

impl<const N: usize> TubName<N> {
    pub fn new() -> Self {
        Self {buf: [0_u8; N]}
    }

    pub fn len(&self) -> usize {
        N
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn to_string(&self) -> String {
        db32enc(&self.buf)
    }

}

impl<const N: usize> fmt::Display for TubName<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}



pub trait Protocol {
    fn new() -> Self;
    fn hash_object(info: u32, data: &[u8]) -> TubHash;
}


pub struct ProtocolZero {

}

impl Protocol for ProtocolZero {
    fn new() -> Self {
        Self {}
    }

    fn hash_object(info: u32, data: &[u8]) -> TubHash {
        let mut h = blake3::Hasher::new();
        h.update(&info.to_le_bytes());
        h.update(data);
        let mut hash: TubHash = [0_u8; TUB_HASH_LEN];
        h.finalize_xof().fill(&mut hash);
        hash
    }
}


/*
struct Store<P: Protocol> {
    protocol: P,
}

impl<P: Protocol> Store<P> {
    fn get(&self, hash: P::Hash, buf: &mut [u8]) {
        self.protocol.hash_leaf(0, buf);
    }

    fn add(&self, buf: &[u8]) -> (P::Hash, bool) {
        (self.protocol.hash_leaf(0, buf), true)
        //P::Hash::new()
    }
}
*/

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use super::*;
    use crate::util::{getrandom, random_object, random_hash};

    const COUNT: usize = 1000;

    #[test]
    fn test_tubname() {
        let mut n = TubName::<30>::new();
        assert_eq!(n.len(), 30);
        assert_eq!(n.as_buf(), [0_u8; 30]);
        assert_eq!(n.as_mut_buf(), [0_u8; 30]);
        assert_eq!(n.to_string(), "333333333333333333333333333333333333333333333333");
        n.as_mut_buf().fill(255);
        assert_eq!(n.len(), 30);
        assert_eq!(n.as_buf(), [255_u8; 30]);
        assert_eq!(n.as_mut_buf(), [255_u8; 30]);
        assert_eq!(n.to_string(), "YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY");
    }

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
    #[should_panic(expected = "Need buffer 39 bytes long, got 38")]
    fn test_hashroot2_panic1() {
        let buf = [0_u8; TAIL_LEN - 1];
        hash_root(&buf);
    }

    #[test]
    #[should_panic(expected = "Need buffer 39 bytes long, got 40")]
    fn test_hashroot2_panic2() {
        let buf = [0_u8; TAIL_LEN + 1];
        hash_root(&buf);
    }

    #[test]
    fn test_hash_root() {
        let mut buf = [0_u8; TAIL_LEN];
        getrandom(&mut buf);
        let mut set: HashSet<TubHash> = HashSet::new();
        for i in 0..buf.len() {
            for v in 0_u8..=255 {
                let mut copy = buf.clone();
                copy[i] = v;
                let newhash = hash_root(&copy);
                set.insert(newhash);
            }
        }
        assert_eq!(set.len(), TAIL_LEN * 255 + 1);
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

