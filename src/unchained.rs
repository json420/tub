//! New blockchain stuffs

use std::ops::Range;



// HASH SIG

pub struct Block<'a, const N: usize> {
    buf: &'a mut [u8],
}

impl<'a, const N: usize> Block<'a, N> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self {buf}
    }

    pub fn size() -> usize {
        N + 64 + N + N + 8
    }

    pub fn hash_range() -> Range<usize> {
        0..N
    }

    pub fn signature_range() -> Range<usize> {
        N..N + 64
    }

    pub fn previous_range() -> Range<usize> {
        let start = N + 64;
        start..start + N
    }

    pub fn payload_range() -> Range<usize> {
        let start = N + 64 + N;
        start..start + N
    }

    pub fn index_range() -> Range<usize> {
        let start = N + 64 + N + N;
        start..start + 8
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    type Block30 = Block<'static, 30>;
    type Block40 = Block<'static, 40>;

    #[test]
    fn test_block_size() {
        assert_eq!(Block30::size(), 162);
        assert_eq!(Block40::size(), 192);
    }

    #[test]
    fn test_block_ranges() {
        assert_eq!(Block30::hash_range(), 0..30);
        assert_eq!(Block30::signature_range(), 30..94);
        assert_eq!(Block30::previous_range(), 94..124);
        assert_eq!(Block30::payload_range(), 124..154);
        assert_eq!(Block30::index_range(), 154..162);

        assert_eq!(Block40::hash_range(), 0..40);
        assert_eq!(Block40::signature_range(), 40..104);
        assert_eq!(Block40::previous_range(), 104..144);
        assert_eq!(Block40::payload_range(), 144..184);
        assert_eq!(Block40::index_range(), 184..192);
    }
}

