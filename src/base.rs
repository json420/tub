use std::mem::size_of;

pub const ABSTRACT_ID_LEN: usize = 15;
pub const OBJECT_ID_LEN: usize = 30;
pub const LEAF_HASH_LEN: usize = 30;

pub type AbstractID = [u8; ABSTRACT_ID_LEN];
pub type ObjectID = [u8; OBJECT_ID_LEN];
pub type LeafHash = [u8; LEAF_HASH_LEN];
pub type LeafHashList = Vec<LeafHash>;

pub type ObjectSize = u64;
pub type OffsetSize = u64;

pub const HEADER_LEN: usize = OBJECT_ID_LEN + size_of::<ObjectSize>();
pub type HeaderBuf = [u8; HEADER_LEN];



pub const LEAF_SIZE: ObjectSize = 8388608;  // 8 MiB leaf size
pub type LeafBuf = Box<[u8; LEAF_SIZE as usize]>;



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lengths() {
        assert_eq!(ABSTRACT_ID_LEN % 5, 0);
        assert_eq!(OBJECT_ID_LEN % 5, 0);
        assert_eq!(LEAF_HASH_LEN % 5, 0);
        assert!(OBJECT_ID_LEN > ABSTRACT_ID_LEN);
        assert_eq!(HEADER_LEN, 38);
    }
}

