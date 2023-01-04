//! Constants and types.

use std::ops;

pub const TUB_ID_LEN: usize = 15;
pub const TUB_HASH_LEN: usize = 30;

pub type TubId = [u8; TUB_ID_LEN];
pub type TubHash = [u8; TUB_HASH_LEN];
pub type TubHashList = Vec<TubHash>;

pub const TAIL_LEN: usize = 9 + TUB_HASH_LEN;  // Size + Type + Payload Hash
pub const HEADER_LEN: usize = TUB_HASH_LEN + TAIL_LEN;

//pub const LEAF_SIZE: u64 = 1048576;  // 1 MiB
//pub const LEAF_SIZE: u64 = 2097152;  // 2 MiB
pub const LEAF_SIZE: u64 = 8388608;  // 8 MiB

pub const ROOT_HASH_RANGE: ops::Range<usize> = 0..TUB_HASH_LEN;
pub const TYPE_INDEX: usize = TUB_HASH_LEN;
pub const SIZE_RANGE: ops::Range<usize> = TUB_HASH_LEN + 1..TUB_HASH_LEN + 9;
pub const PAYLOAD_HASH_RANGE: ops::Range<usize> = TUB_HASH_LEN + 9..2 * TUB_HASH_LEN + 9;

pub const TAIL_RANGE: ops::Range<usize> = TUB_HASH_LEN..HEADER_LEN;

pub const BLOCK_PREVIOUS_HASH_RANGE: ops::Range<usize> = TUB_HASH_LEN..2 * TUB_HASH_LEN;
pub const BLOCK_PAYLOAD_HASH_RANGE: ops::Range<usize> = 2 * TUB_HASH_LEN..3 * TUB_HASH_LEN;


pub const BLOCK_SIGNATURE_RANGE: ops::Range<usize> = 0..64;
pub const BLOCK_SIGNABLE_RANGE: ops::Range<usize> = 64..172;

pub const BLOCK_PUBKEY_RANGE: ops::Range<usize> = 64..96;
pub const BLOCK_PREVIOUS_RANGE: ops::Range<usize> = 96..126;
pub const BLOCK_TYPE_INDEX: usize = 126;  // 126..127
pub const BLOCK_COUNTER_RANGE: ops::Range<usize> = 127..135;
pub const BLOCK_TIMESTAMP_RANGE: ops::Range<usize> = 135..143;
pub const BLOCK_PAYLOAD_RANGE: ops::Range<usize> = 143..173;
pub const BLOCK_LEN: usize = 173;

pub const OBJECT_MAX_SIZE: usize = 16777216;
pub const OBJECT_HEADER_LEN: usize = TUB_HASH_LEN + 4;
pub const OBJECT_HASH_RANGE: ops::Range<usize> = 0..TUB_HASH_LEN;
pub const OBJECT_INFO_RANGE: ops::Range<usize> = TUB_HASH_LEN..OBJECT_HEADER_LEN;
pub const OBJECT_HEADER_RANGE: ops::Range<usize> = 0..OBJECT_HEADER_LEN;

pub const DOTDIR: &str = ".tub";
pub const PACKFILE: &str = "append.tub";
pub const OBJECTDIR: &str = "objects";
pub const PARTIALDIR: &str = "partial";
pub const TMPDIR: &str = "tmp";
pub const README: &str = "REAMDE.txt";  // The REAMDE file

pub static README_CONTENTS: &[u8] = b"Hello from Bathtub DB!

What's even more relaxing than a Couch?  A Bathtub!
";


#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ObjectType {
    Data,
    Tree,
    Block,
    Commit,
}

impl From<u8> for ObjectType {
    fn from(item: u8) -> Self {
        match item {
            0 => Self::Data,
            1 => Self::Tree,
            2 => Self::Block,
            3 => Self::Commit,
            _ => panic!("Unknown ObjectType: {}", item),
        }
    }
}


#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BlockType {
    Configure,
    Commit,
}

impl From<u8> for BlockType {
    fn from(item: u8) -> Self {
        match item {
            0 => Self::Configure,
            1 => Self::Commit,
            _ => panic!("Unknown BlockType: {}", item),
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lengths() {
        assert_eq!(TUB_ID_LEN % 5, 0);
        assert_eq!(TUB_HASH_LEN % 5, 0);
        assert_eq!(TUB_HASH_LEN % 5, 0);
        assert!(TUB_HASH_LEN > TUB_ID_LEN);
        assert_eq!(HEADER_LEN, 69);
    }

    #[test]
    fn test_ranges() {
         // Yes, these break if TUB_HASH_LEN changes, but just to see them clearly:
         assert_eq!(ROOT_HASH_RANGE, 0..30);
         assert_eq!(TYPE_INDEX, 30);
         assert_eq!(SIZE_RANGE, 31..39);
         assert_eq!(PAYLOAD_HASH_RANGE, 39..69);
    }

    #[test]
    fn test_objtype() {
        for k in 0..3 {
            let ot: ObjectType = k.into();
            assert_eq!(ot as u8, k);
        }
        assert_eq!(ObjectType::Data as u8, 0);
        assert_eq!(ObjectType::Data, 0.into());
        assert_eq!(ObjectType::Tree as u8, 1);
        assert_eq!(ObjectType::Tree, 1.into());
        assert_eq!(ObjectType::Block as u8, 2);
        assert_eq!(ObjectType::Block, 2.into());
        assert_eq!(ObjectType::Commit as u8, 3);
        assert_eq!(ObjectType::Commit, 3.into());
    }

    #[test]
    #[should_panic(expected = "Unknown ObjectType: 4")]
    fn test_objtype_panic1() {
        let _kind: ObjectType = 4.into();
    }

    #[test]
    #[should_panic(expected = "Unknown ObjectType: 255")]
    fn test_objtype_panic2() {
        let _kind: ObjectType = 255.into();
    }

    #[test]
    fn test_blocktype() {
        for k in 0..2 {
            let ot: BlockType = k.into();
            assert_eq!(ot as u8, k);
        }
        assert_eq!(BlockType::Configure as u8, 0);
        assert_eq!(BlockType::Configure, 0.into());
        assert_eq!(BlockType::Commit as u8, 1);
        assert_eq!(BlockType::Commit, 1.into());
    }

    #[test]
    #[should_panic(expected = "Unknown BlockType: 2")]
    fn test_blockype_panic1() {
        let _kind: BlockType = 2.into();
    }

    #[test]
    #[should_panic(expected = "Unknown BlockType: 255")]
    fn test_block_type_panic2() {
        let _kind: BlockType = 255.into();
    }
}

