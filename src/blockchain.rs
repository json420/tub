//! Super cool crypto in here, yo! 💵 💵 💵

use std::{fs, io};
use std::os::unix::fs::FileExt;
use std::io::prelude::*;
use sodiumoxide::crypto::sign;
use blake3;
use crate::chaos::Name;


/*
Rough cycles for ed5519 to sign/verify:
Sign:    87548
Verify: 273364

https://ed25519.cr.yp.to/

The idea is to use a signed block chain in which the key rotates each signature
(given key is only used once?).  The key is used for short term access control,
whereas the hash chain provides long term history preservation (even if the
key is compromised).

You can think of a Tub blockchain like a namespace or an identity.

ROOT, SIGNATURE, PUBKEY, PREVIOUS, COUNTER, TIMESTAMP, PAYLOAD_HASH
*/



// SIG PREVIOUS PAYLOAD
pub struct WriteBlock<'a, const N: usize> {
    inner: &'a mut [u8],
    sk: sign::SecretKey,
}

impl<'a, const N: usize> WriteBlock<'a, N> {
    pub fn new(inner: &'a mut [u8], sk: sign::SecretKey) -> Self{
        Self {inner: inner, sk: sk}
    }

    pub fn into_sk(self) -> sign::SecretKey {
        self.sk
    }

    pub fn sign(&mut self) -> sign::Signature {
        let sig = sign::sign_detached(self.as_signed(), &self.sk);
        self.set_signature(sig.as_ref());
        sig
    }

    pub fn as_signed(&self) -> &[u8] {
        &self.inner[64..64 + N * 2]
    }

    pub fn set_signature(&mut self, value: &[u8]) {
        self.inner[0..64].copy_from_slice(value);
    }

    pub fn set_previous(&mut self, value: &[u8]) {
        self.inner[64..64 + N].copy_from_slice(value);
    }

    pub fn set_payload(&mut self, value: &[u8]) {
        self.inner[64 + N..64 + N * 2].copy_from_slice(value);
    }
}

// pub fn verify_detached(sig: &Signature, m: &[u8], pk: &PublicKey) -> bool
pub struct ReadBlock<'a, const N: usize> {
    inner: &'a [u8],
    pk: sign::PublicKey,
}

impl<'a, const N: usize> ReadBlock<'a, N> {
    pub fn new(inner: &'a [u8], pk: sign::PublicKey) -> Self{
        Self {inner: inner, pk: pk}
    }

    pub fn is_valid(&self) -> bool {
        sign::verify_detached(&self.signature(), self.as_signed(), &self.pk)
    }

    pub fn signature(&self) -> sign::Signature {
        sign::Signature::try_from(&self.inner[0..64]).unwrap()
    }

    pub fn previous(&self) -> Name<N> {
        Name::from(&self.inner[64..64 + N])
    }

    pub fn payload(&self) -> Name<N> {
        Name::from(&self.inner[64 + N..64 + N * 2])
    }

    pub fn as_signed(&self) -> &[u8] {
        &self.inner[64..64 + N * 2]
    }
}




#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use crate::util::getrandom;
    use crate::helpers::flip_bit_in;
    use crate::chaos::{DefaultName, DefaultObject};
    use super::*;

    #[test]
    fn test_write_block() {
        let mut obj = DefaultObject::new();
        obj.reset(124, 0);
        let (pk, sk) = sign::gen_keypair();
        let mut writer: WriteBlock<30> = WriteBlock::new(obj.as_mut_data(), sk);
        let mut name = DefaultName::new();
        name.randomize();
        writer.set_previous(name.as_buf());
        name.randomize();
        writer.set_payload(name.as_buf());
        let sig = writer.sign();
        assert!(sign::verify(obj.as_data(), &pk).is_ok());
    }

    #[test]
    fn test_read_block() {
        let mut obj = DefaultObject::new();
        obj.reset(124, 0);
        let (pk, sk) = sign::gen_keypair();
        let reader: ReadBlock<30> = ReadBlock::new(obj.as_data(), pk);
        assert!(! reader.is_valid());
    }

    #[test]
    fn test_ed25519() {
        let (pk, sk) = sign::gen_keypair();
        let data = b"some data";
        let signed_data = sign::sign(data, &sk);
        let verified_data = sign::verify(&signed_data, &pk).unwrap();
        assert_eq!(data, &verified_data[..]);
    }
}

