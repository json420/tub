//! Object hashing protocol.



// https://bazaar.launchpad.net/~dmedia/filestore/trunk/view/head:/filestore/protocols.py

/*  FIXME: Skein probably provides better performance and a better security
    margin than Blake2b, so we should strongly consider Skein.
*/
use std::fmt;
use blake3;
use crate::base::*;
use crate::dbase32::db32enc_str;


pub fn hash(buf: &[u8]) -> RootInfo {
    assert!(buf.len() as u64 <= LEAF_SIZE);
    let leaf = hash_leaf(0, buf);
    hash_root(buf.len() as u64, vec![leaf.hash])
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

pub fn hash_leaf_into(index: u64, data: &[u8], hash: &mut [u8])
{
    assert_eq!(hash.len(), TUB_HASH_LEN);
    let mut h = blake3::Hasher::new();
    h.update(&index.to_le_bytes());
    h.update(data);
    h.finalize_xof().fill(hash);
}

pub fn hash_leaf(index: u64, data: &[u8]) -> LeafInfo {
    let mut hash: TubHash = [0_u8; TUB_HASH_LEN];
    hash_leaf_into(index, data, &mut hash);
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

    pub fn small(&self) -> bool {
        self.size <= LEAF_SIZE
    }

    pub fn large(&self) -> bool {
        self.size > LEAF_SIZE
    }
}

impl fmt::Display for RootInfo {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{}", db32enc_str(&self.hash))
    }
}

pub fn hash_root_raw(data: &[u8]) -> TubHash {
    let mut h = blake3::Hasher::new();
    h.update(data);
    let mut hash: TubHash = [0_u8; TUB_HASH_LEN];
    h.finalize_xof().fill(&mut hash);
    hash
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


pub struct TubTop {
    index: u64,
    total: u64,
    buf: Vec<u8>,
}

impl TubTop {
    pub fn new() -> Self {
        let mut buf = Vec::with_capacity(HEADER_LEN + TUB_HASH_LEN);
        buf.resize(HEADER_LEN, 0);
        Self {index: 0, total: 0, buf: buf}
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_buf(self) -> Vec<u8> {
        self.buf
    }

    pub fn hash(&self) -> TubHash {
        self.buf[0..TUB_HASH_LEN].try_into().expect("oops")
    }

    pub fn size(&self) -> u64 {
        u64::from_le_bytes(
            self.buf[TUB_HASH_LEN..HEADER_LEN].try_into().expect("oops")
        )
    }

    pub fn leaf_hash(&self, index: usize) -> TubHash {
        assert_eq!(self.size(), 0);
        let start = HEADER_LEN + (index * TUB_HASH_LEN);
        let stop = start + TUB_HASH_LEN;
        self.buf[start..stop].try_into().expect("oops")
    }

    pub fn hash_next_leaf(&mut self, data: &[u8]) {
        assert!(data.len() > 0 && data.len() <= LEAF_SIZE as usize);
        self.buf.resize(self.buf.len() + TUB_HASH_LEN, 0);
        let start = self.buf.len() - TUB_HASH_LEN;
        hash_leaf_into(self.index, data, &mut self.buf[start..]);
        self.index += 1;
        self.total += data.len() as u64;
    }

    pub fn finalize(&mut self) {
        assert_eq!(self.size(), 0);
        self.buf.splice(TUB_HASH_LEN..HEADER_LEN, self.total.to_le_bytes());
        let hash = hash_root_raw(&self.buf[TUB_HASH_LEN..]);
        self.buf.splice(0..TUB_HASH_LEN, hash);
    }

    pub fn is_large(&self) -> bool {
        assert_ne!(self.size(), 0);
        self.size() > LEAF_SIZE
    }

    pub fn is_small(&self) -> bool {
        ! self.is_large()
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::{random_small_object, random_hash};
    use crate::dbase32::db32enc_str;

    #[test]
    fn test_hash() {
        let root = hash(b"Federation44");
        assert_eq!(&db32enc_str(&root.hash),
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
