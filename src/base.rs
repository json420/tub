//! Constants and types.

use std::mem::size_of;
use std::ops;

pub const TUB_ID_LEN: usize = 15;
pub const TUB_HASH_LEN: usize = 30;

pub type TubId = [u8; TUB_ID_LEN];
pub type TubHash = [u8; TUB_HASH_LEN];
pub type TubHashList = Vec<TubHash>;

pub const HEADER_LEN: usize = 2 * TUB_HASH_LEN + 8;

//pub const LEAF_SIZE: u64 = 1048576;  // 1 MiB
//pub const LEAF_SIZE: u64 = 2097152;  // 2 MiB
pub const LEAF_SIZE: u64 = 8388608;  // 8 MiB

pub const ROOT_HASH_RANGE: ops::Range<usize> = 0..TUB_HASH_LEN;
pub const SIZE_RANGE: ops::Range<usize> = TUB_HASH_LEN..TUB_HASH_LEN + 8;
pub const PAYLOAD_HASH_RANGE: ops::Range<usize> = (8 + TUB_HASH_LEN)..(8 + TUB_HASH_LEN * 2);

pub const TAIL_RANGE: ops::Range<usize> = TUB_HASH_LEN..HEADER_LEN - TUB_HASH_LEN;


pub const DOTDIR: &str = ".bathtub_db";
pub const PACKFILE: &str = "bathtub.db";
pub const OBJECTDIR: &str = "objects";
pub const PARTIALDIR: &str = "partial";
pub const TMPDIR: &str = "tmp";
pub const README: &str = "REAMDE.txt";  // The REAMDE file

pub static README_CONTENTS: &[u8] = b"Hello from Bathtub DB!

What's even more relaxing than a Couch?  A Bathtub!
";



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lengths() {
        assert_eq!(TUB_ID_LEN % 5, 0);
        assert_eq!(TUB_HASH_LEN % 5, 0);
        assert_eq!(TUB_HASH_LEN % 5, 0);
        assert!(TUB_HASH_LEN > TUB_ID_LEN);
        assert_eq!(HEADER_LEN, 68);
    }
}

