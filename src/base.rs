//! Constants and types.

use std::mem::size_of;

pub const ABSTRACT_ID_LEN: usize = 15;
pub const TUB_HASH_LEN: usize = 30;

pub type AbstractID = [u8; ABSTRACT_ID_LEN];
pub type TubHash = [u8; TUB_HASH_LEN];
pub type TubHashList = Vec<TubHash>;

pub type ObjectSize = u64;
pub type OffsetSize = u64;
pub type LeafIndex = u64;

pub const HEADER_LEN: usize = TUB_HASH_LEN + size_of::<ObjectSize>();
pub type HeaderBuf = [u8; HEADER_LEN];

//pub const LEAF_SIZE: ObjectSize = 2097152;  // 2 MiB leaf size
pub const LEAF_SIZE: ObjectSize = 8388608;  // 8 MiB leaf size
pub type LeafBuf = Box<[u8; LEAF_SIZE as usize]>;


pub const DOTDIR: &str = ".bathtub_db";
pub const PACKFILE: &str = "bathtub.db";
pub const OBJECTDIR: &str = "objects";
pub const PARTIALDIR: &str = "partial";
pub const TMPDIR: &str = "tmp";
pub const README: &str = "REAMDE.txt";  // The REAMDE file

pub static README_CONTENTS: &[u8] = b"Hello from Bathtub DB!

What's even more relaxing than a Couch?  A Bathtub!
";


pub struct ObjectInfo {
    id: TubHash,
    size: ObjectSize,
    leaf_hashes: TubHashList,
}

impl ObjectInfo
{
    pub fn new(id: TubHash, size: ObjectSize, leaf_hashes: TubHashList) -> Self
    {
        Self {
            id: id,
            size: size,
            leaf_hashes: leaf_hashes,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lengths() {
        assert_eq!(ABSTRACT_ID_LEN % 5, 0);
        assert_eq!(TUB_HASH_LEN % 5, 0);
        assert_eq!(TUB_HASH_LEN % 5, 0);
        assert!(TUB_HASH_LEN > ABSTRACT_ID_LEN);
        assert_eq!(HEADER_LEN, 38);
    }
}

