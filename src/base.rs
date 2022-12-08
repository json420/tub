//! Constants and types.

use std::ops;

pub const TUB_ID_LEN: usize = 15;
pub const TUB_HASH_LEN: usize = 30;

pub type TubId = [u8; TUB_ID_LEN];
pub type TubHash = [u8; TUB_HASH_LEN];
pub type TubHashList = Vec<TubHash>;

pub const HEADER_LEN: usize = 2 * TUB_HASH_LEN + 9;

//pub const LEAF_SIZE: u64 = 1048576;  // 1 MiB
//pub const LEAF_SIZE: u64 = 2097152;  // 2 MiB
pub const LEAF_SIZE: u64 = 8388608;  // 8 MiB

pub const ROOT_HASH_RANGE: ops::Range<usize> = 0..TUB_HASH_LEN;
pub const SIZE_RANGE: ops::Range<usize> = TUB_HASH_LEN..TUB_HASH_LEN + 8;
pub const TYPE_RANGE: ops::Range<usize> = TUB_HASH_LEN + 8..TUB_HASH_LEN + 9;
pub const PAYLOAD_HASH_RANGE: ops::Range<usize> = TUB_HASH_LEN + 9..2 * TUB_HASH_LEN + 9;

pub const TAIL_RANGE: ops::Range<usize> = TUB_HASH_LEN..HEADER_LEN;


pub const DOTDIR: &str = ".bathtub_db";
pub const PACKFILE: &str = "bathtub.db";
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
}

impl From<u8> for ObjectType {
    fn from(item: u8) -> Self {
        match item {
            0 => Self::Data,
            1 => Self::Tree,
            _ => panic!("Unknown ObjectType: {}", item),
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
         assert_eq!(SIZE_RANGE, 30..38);
         assert_eq!(TYPE_RANGE, 38..39);
         assert_eq!(PAYLOAD_HASH_RANGE, 39..69);
    }

    #[test]
    fn test_objtype() {
        for k in 0..2 {
            let ot: ObjectType = k.into();
            assert_eq!(ot as u8, k);
        }
        assert_eq!(ObjectType::Data as u8, 0);
        assert_eq!(ObjectType::Data, 0.into());
        assert_eq!(ObjectType::Tree as u8, 1);
        assert_eq!(ObjectType::Tree, 1.into());
    }

    #[test]
    #[should_panic(expected = "Unknown ObjectType: 2")]
    fn test_objtype_panic1() {
        let _kind: ObjectType = 2.into();
    }

    #[test]
    #[should_panic(expected = "Unknown ObjectType: 255")]
    fn test_objtype_panic2() {
        let _kind: ObjectType = 255.into();
    }
}

