//! Super cool crypto in here, yo! 💵 💵 💵

use std::os::unix::fs::FileExt;
use std::io::prelude::*;
use sodiumoxide::crypto::sign;
use blake3;
use crate::chaos::Name;


/*
Rough cycles for ed25519 to sign/verify:
Sign:    87548
Verify: 273364

https://ed25519.cr.yp.to/

The idea is to use a signed block chain.  The key is used for short term access
control, whereas the hash chain provides long term history preservation (even if
the key is compromised)

You can think of a Tub blockchain like a namespace or an identity.

ROOT, SIGNATURE, PUBKEY, PREVIOUS, COUNTER, TIMESTAMP, PAYLOAD_HASH
*/


// SIG PREVIOUS PAYLOAD
pub struct Block<'a, const N: usize> {
    inner: &'a mut [u8],
    pk: sign::PublicKey,
}

impl<'a, const N: usize> Block<'a, N> {
    pub fn new(inner: &'a mut [u8], pk: sign::PublicKey) -> Self{
        Self {inner: inner, pk: pk}
    }

    pub fn into_pk(self) -> sign::PublicKey {
        self.pk
    }

    pub fn sign(&mut self, sk: &sign::SecretKey) -> sign::Signature {
        let sig = sign::sign_detached(self.as_signed(), sk);
        self.set_signature(sig.as_ref());
        sig
    }

    pub fn as_signed(&self) -> &[u8] {
        &self.inner[64..64 + N * 2]
    }

    pub fn verify(&self) -> bool {
        if let Ok(sig) = self.signature() {
            sign::verify_detached(&sig, self.as_signed(), &self.pk)
        }
        else {
            false
        }
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

    pub fn signature(&self) -> Result<sign::Signature, sign::Error> {
        sign::Signature::try_from(&self.inner[0..64])
    }

    pub fn previous(&self) -> Name<N> {
        Name::from(&self.inner[64..64 + N])
    }

    pub fn payload(&self) -> Name<N> {
        Name::from(&self.inner[64 + N..64 + N * 2])
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
    fn test_block_set_get() {
        let (pk, sk) = sign::gen_keypair();
        let mut buf = [0_u8; 124];
        let mut block: Block<30> = Block::new(&mut buf, pk);
        assert_eq!(block.signature().unwrap().as_ref(), [0; 64]);
        assert_eq!(block.previous().as_buf(), [0; 30]);
        assert_eq!(block.payload().as_buf(), [0; 30]);

        let sig = sign::sign_detached(b"Just for testing and fun", &sk);
        block.set_signature(sig.as_ref());
        assert_eq!(block.signature().unwrap(), sig);

        let mut previous = DefaultName::new();
        previous.randomize();
        block.set_previous(previous.as_buf());
        assert_ne!(block.previous().as_buf(), [0; 64]);
        assert_eq!(block.previous(), previous);

        let mut payload = DefaultName::new();
        payload.randomize();
        block.set_payload(payload.as_buf());
        assert_ne!(block.payload().as_buf(), [0; 64]);
        assert_eq!(block.payload(), payload);
    }

    #[test]
    fn test_block_verify() {
        let (pk, sk) = sign::gen_keypair();
        let mut buf = [0_u8; 124];
        let mut block: Block<30> = Block::new(&mut buf, pk);
        assert!(! block.verify());

        let mut name = DefaultName::new();
        name.randomize();
        block.set_previous(name.as_buf());
        name.randomize();
        block.set_payload(name.as_buf());
        block.sign(&sk);

        let pk = block.into_pk();
        for bit in 0..buf.len() * 8 {
            let mut copy = buf.clone();
            flip_bit_in(&mut copy, bit);
            let block: Block<30> = Block::new(&mut copy, pk);
            assert!(! block.verify());
        }
        
    }

    #[test]
    fn test_block_with_object() {
        let mut obj = DefaultObject::new();
        obj.reset(124, 0);
        let (pk, sk) = sign::gen_keypair();
        let mut block: Block<30> = Block::new(obj.as_mut_data(), pk);
        assert!(! block.verify());

        let mut name = DefaultName::new();
        name.randomize();
        block.set_previous(name.as_buf());
        name.randomize();
        block.set_payload(name.as_buf());
        assert!(! block.verify());
        let sig = block.sign(&sk);
        assert!(block.verify());
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

