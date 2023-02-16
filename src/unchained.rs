//! New blockchain stuffs

use std::ops::Range;
use sodiumoxide::crypto::sign;



// HASH SIG

pub struct Math<const N: usize> {}


impl<const N: usize> Math<N> {
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


pub struct Block<'a, const N: usize> {
    buf: &'a mut [u8],
}

impl<'a, const N: usize> Block<'a, N> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self {buf}
    }

    // Note not all signature values are structurally valid
    pub fn signature(&self) -> Result<sign::Signature, sign::Error> {
        let range = Math::<N>::signature_range();
        sign::Signature::try_from(&self.buf[range])
    }
}


pub struct Read<'a> {
    buf: &'a [u8],
}

impl<'a> Read<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {buf}
    }
}

pub struct Write<'a> {
    buf: &'a mut [u8],
}

impl<'a> Write<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self {buf}
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    type Math30 = Math<30>;
    type Math40 = Math<40>;

    #[test]
    fn test_stuff() {
        let mut buf = [0_u8; 30];
        let r = Read::new(&buf);
        let w = Write::new(&mut buf);
    }

    #[test]
    fn test_math() {
        assert_eq!(Math30::size(), 162);
        assert_eq!(Math30::hash_range(), 0..30);
        assert_eq!(Math30::signature_range(), 30..94);
        assert_eq!(Math30::previous_range(), 94..124);
        assert_eq!(Math30::payload_range(), 124..154);
        assert_eq!(Math30::index_range(), 154..162);

        assert_eq!(Math40::size(), 192);
        assert_eq!(Math40::hash_range(), 0..40);
        assert_eq!(Math40::signature_range(), 40..104);
        assert_eq!(Math40::previous_range(), 104..144);
        assert_eq!(Math40::payload_range(), 144..184);
        assert_eq!(Math40::index_range(), 184..192);
    }
}

