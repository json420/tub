//! Super cool crypto in here, yo! 💵 💵 💵

use std::{io, fs};
use std::io::prelude::*;
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


/*
The KeyBlock is a self signed special block that starts the chain.  It does not
have `previous` nor `payload` fields (unlike `Block`).
*/

// SIG PUBKEY
pub struct Header {
    buf: [u8; 96],
}

impl Header {
    pub fn new() -> Self {
        Self {buf: [0; 96]}
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn generate(&mut self) {
        let (pk, sk) = sign::gen_keypair();
        self.sign(&sk);
    }

    pub fn sign(&mut self, sk: &sign::SecretKey) -> sign::Signature {
        let pk = sk.public_key();
        let sig = sign::sign_detached(pk.as_ref(), sk);
        self.set_signature(sig.as_ref());
        self.set_pubkey(pk.as_ref());
        sig
    }

    pub fn set_signature(&mut self, value: &[u8]) {
        self.buf[0..64].copy_from_slice(value);
    }

    pub fn set_pubkey(&mut self, value: &[u8]) {
        self.buf[64..96].copy_from_slice(value);
    }

    // Note not all signature values are structurally valid
    pub fn signature(&self) -> Result<sign::Signature, sign::Error> {
        sign::Signature::try_from(&self.buf[0..64])
    }

    pub fn pubkey(&self) -> sign::PublicKey {
        sign::PublicKey::from_slice(&self.buf[64..96]).unwrap()
    }

    pub fn verify(&self) -> bool {
        if let Ok(sig) = self.signature() {
            sign::verify_detached(&sig, &self.buf[64..96], &self.pubkey())
        }
        else {
            false
        }
    }
}


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

    // Note not all signature values are structurally valid
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


pub struct Chain<const N: usize> {
    pk: sign::PublicKey,
    file: fs::File,
}

impl<const N: usize> Chain<N> {
    pub fn verify_chain(&mut self) -> io::Result<bool> {
        let mut br = io::BufReader::new(self.file.try_clone()?);
        br.seek(io::SeekFrom::Start(0))?;
        Ok(false)
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
    fn test_block_chain() {
    }
/*
    #[test]
    fn test_keyblock() {
        let mut buf = [0_u8; 96];
        let mut kb = KeyBlock::new(&mut buf);
        assert_eq!(kb.pubkey().as_ref(), [0; 32]);
        assert_eq!(kb.signature().unwrap().as_ref(), [0; 64]);
        assert!(! kb.verify());
        let (pk, sk) = sign::gen_keypair();
        let sig = kb.sign(&sk);
        assert!(kb.verify());
        assert_eq!(kb.signature().unwrap(), sig);
        assert_eq!(kb.pubkey(), pk);

        let (pk, sk) = sign::gen_keypair();
        let sig = sign::sign_detached(pk.as_ref(), &sk);
        let mut buf = [0_u8; 96];
        buf[0..64].copy_from_slice(sig.as_ref());
        buf[64..96].copy_from_slice(pk.as_ref());
        let mut kb = KeyBlock::new(&mut buf);
        assert!(kb.verify());
        assert_eq!(kb.signature().unwrap(), sig);
        assert_eq!(kb.pubkey(), pk);

        for bit in 0..buf.len() * 8 {
            let mut copy = buf.clone();
            flip_bit_in(&mut copy, bit);
            let kb = KeyBlock::new(&mut copy);
            assert!(! kb.verify());
        }
    }

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
        let mut copy = buf.clone();
        let block: Block<30> = Block::new(&mut copy, pk);
        assert!(block.verify());
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
    */

}

