//! Constants and types.

use std::mem::size_of;

pub const TUB_ID_LEN: usize = 15;
pub const TUB_HASH_LEN: usize = 30;

pub type TubId = [u8; TUB_ID_LEN];
pub type TubHash = [u8; TUB_HASH_LEN];
pub type TubHashList = Vec<TubHash>;

pub const HEADER_LEN: usize = TUB_HASH_LEN + size_of::<u64>();
pub type HeaderBuf = [u8; HEADER_LEN];

//pub const LEAF_SIZE: u64 = 2097152;  // 2 MiB leaf size
pub const LEAF_SIZE: u64 = 8388608;  // 8 MiB leaf size



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
        assert_eq!(HEADER_LEN, 38);
    }
}

