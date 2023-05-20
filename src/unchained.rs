//! New blockchain stuffs

use std::ops::Range;
use rand::rngs::OsRng;
use ed25519_dalek::{
    SigningKey,
    Signer,
    Signature,
    SignatureError,
    VerifyingKey,
    Verifier,
};



/*
First pass:

N    64        32     N    N        N
HASH SIGNATURE PUBKEY NEXT PREVIOUS PAYLOAD
HASH SIGNATURE PUBKEY NEXT



Then probably add:
N    64        32     8       8         N  N    N        N
HASH SIGNATURE PUBKEY COUNTER TIMESTAMP ID NEXT PREVIOUS PAYLOAD
HASH SIGNATURE PUBKEY NEXT
*/


pub struct Math<const N: usize> {}

impl<const N: usize> Math<N> {
    pub fn hash_range() -> Range<usize> {
        0..N
    }

    pub fn signature_range() -> Range<usize> {
        N..N + 64
    }

    pub fn pubkey_range() -> Range<usize> {
        let start = N + 64;
        start..start + 32
    }

    pub fn next_range() -> Range<usize> {
        let start = N + 96;
        start..start + N
    }

    pub fn previous_range() -> Range<usize> {
        let start = 2 * N + 96;
        start..start + N
    }

    pub fn payload_range() -> Range<usize> {
        let start = 3 * N + 96;
        start..start + N
    }

    pub fn size() -> usize {
        4 * N + 96
    }
}


pub struct Block<'a, const N: usize> {
    buf: &'a [u8],
}

impl<'a, const N: usize> Block<'a, N> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {buf}
    }

    pub fn as_hash(&self) -> &[u8] {
        &self.buf[Math::<N>::hash_range()]
    }

    pub fn as_signature(&self) -> &[u8] {
        &self.buf[Math::<N>::signature_range()]
    }

    pub fn as_pubkey(&self) -> &[u8] {
        &self.buf[Math::<N>::pubkey_range()]
    }

    pub fn as_next(&self) -> &[u8] {
        &self.buf[Math::<N>::next_range()]
    }

    pub fn as_previous(&self) -> &[u8] {
        &self.buf[Math::<N>::previous_range()]
    }

    pub fn as_payload(&self) -> &[u8] {
        &self.buf[Math::<N>::payload_range()]
    }
}


pub struct MutBlock<'a, const N: usize> {
    buf: &'a mut [u8],
}

impl<'a, const N: usize> MutBlock<'a, N> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self {buf}
    }

    pub fn as_hash(&mut self) -> &mut [u8] {
        &mut self.buf[Math::<N>::hash_range()]
    }

    pub fn as_signature(&mut self) -> &mut [u8] {
        &mut self.buf[Math::<N>::signature_range()]
    }

    pub fn as_pubkey(&mut self) -> &mut [u8] {
        &mut self.buf[Math::<N>::pubkey_range()]
    }

    pub fn as_next(&mut self) -> &mut [u8] {
        &mut self.buf[Math::<N>::next_range()]
    }

    pub fn as_previous(&mut self) -> &mut [u8] {
        &mut self.buf[Math::<N>::previous_range()]
    }

    pub fn as_payload(&mut self) -> &mut [u8] {
        &mut self.buf[Math::<N>::payload_range()]
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
        assert_eq!(Math30::hash_range(), 0..30);
        assert_eq!(Math30::signature_range(), 30..94);
        assert_eq!(Math30::pubkey_range(), 94..126);
        assert_eq!(Math30::next_range(), 126..156);
        assert_eq!(Math30::previous_range(), 156..186);
        assert_eq!(Math30::payload_range(), 186..216);

        assert_eq!(Math40::hash_range(), 0..40);
        assert_eq!(Math40::signature_range(), 40..104);
        assert_eq!(Math40::pubkey_range(), 104..136);
        assert_eq!(Math40::next_range(), 136..176);
        assert_eq!(Math40::previous_range(), 176..216);
        assert_eq!(Math40::payload_range(), 216..256);
    }
}

