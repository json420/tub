//! Super cool crypto in here, yo! 💵 💵 💵

use std::{io, fs};
use std::ops::Range;
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


fn compute_hash(payload: &[u8]) -> Name<30> {
    let mut h = blake3::Hasher::new();
    h.update(payload);
    let mut hash: Name<30> = Name::new();
    h.finalize_xof().fill(hash.as_mut_buf());
    hash
}


const HASH_RANGE: Range<usize> = 0..30;
const SIGNATURE_RANGE: Range<usize> = 30..94;

const HEADER_LEN: usize = 126;
const HEADER_PUBKEY_RANGE: Range<usize> = 94..126;
const HEADER_HASHED_RANGE: Range<usize> = 30..126;

// HASH SIG PUBKEY
pub struct Header {
    buf: [u8; HEADER_LEN],
}

impl Header {
    pub fn new() -> Self {
        Self {buf: [0; HEADER_LEN]}
    }

    pub fn generate() -> (sign::SecretKey, Self) {
        let (pk, sk) = sign::gen_keypair();
        let mut me = Self::new();
        me.sign(&sk);
        (sk, me)
    }

    pub fn compute(&self) -> Name<30> {
        compute_hash(&self.buf[HEADER_HASHED_RANGE])
    }

    pub fn sign(&mut self, sk: &sign::SecretKey) -> sign::Signature {
        let pk = sk.public_key();
        let sig = sign::sign_detached(pk.as_ref(), sk);
        self.set_signature(&sig);
        self.set_pubkey(&pk);
        sig
    }

    pub fn verify(&self) -> bool {
        if let Ok(sig) = self.signature() {
            sign::verify_detached(&sig, &self.buf[64..96], &self.pubkey())
        }
        else {
            false
        }
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn hash(&self) -> Name<30> {
        Name::from(&self.buf[HASH_RANGE])
    }

    pub fn set_hash(&mut self, hash: &Name<30>) {
        self.buf[HASH_RANGE].copy_from_slice(hash.as_buf());
    }

    // Note not all signature values are structurally valid
    pub fn signature(&self) -> Result<sign::Signature, sign::Error> {
        sign::Signature::try_from(&self.buf[SIGNATURE_RANGE])
    }

    pub fn set_signature(&mut self, sig: &sign::Signature) {
        self.buf[SIGNATURE_RANGE].copy_from_slice(sig.as_ref());
    }

    pub fn pubkey(&self) -> sign::PublicKey {
        sign::PublicKey::from_slice(&self.buf[HEADER_PUBKEY_RANGE]).unwrap()
    }

    pub fn set_pubkey(&mut self, pk: &sign::PublicKey) {
        self.buf[HEADER_PUBKEY_RANGE].copy_from_slice(pk.as_ref());
    }
}

const BLOCK_LEN: usize = 154;
const BLOCK_PREVIOUS_RANGE: Range<usize> = 94..124;
const BLOCK_PAYLOAD_RANGE: Range<usize> = 124..154;
const BLOCK_SIGNED_RANGE: Range<usize> = 94..154;
const BLOCK_HASHED_RANGE: Range<usize> = 30..154;

// 30    64     30       30
// HASH  SIG    PREVIOUS PAYLOAD
// 0..30 30..94 94..124  124..154
pub struct Block {
    buf: [u8; BLOCK_LEN],
    pk: sign::PublicKey,
}

impl Block {
    pub fn new(pk: sign::PublicKey) -> Self{
        Self {buf: [0; BLOCK_LEN], pk: pk}
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn compute(&self) -> Name<30> {
        compute_hash(&self.buf[BLOCK_HASHED_RANGE])
    }

    pub fn sign(&mut self, sk: &sign::SecretKey) -> sign::Signature {
        let sig = sign::sign_detached(self.as_signed(), sk);
        self.set_signature(sig.as_ref());
        sig
    }

    pub fn as_signed(&self) -> &[u8] {
        &self.buf[BLOCK_SIGNED_RANGE]
    }

    pub fn verify(&self) -> bool {
        if let Ok(sig) = self.signature() {
            sign::verify_detached(&sig, self.as_signed(), &self.pk)
        }
        else {
            false
        }
    }

    pub fn hash(&self) -> Name<30> {
        Name::from(&self.buf[HASH_RANGE])
    }

    pub fn set_hash(&mut self, value: &[u8]) {
        self.buf[HASH_RANGE].copy_from_slice(value);
    }

    // Note not all signature values are structurally valid
    pub fn signature(&self) -> Result<sign::Signature, sign::Error> {
        sign::Signature::try_from(&self.buf[SIGNATURE_RANGE])
    }

    pub fn set_signature(&mut self, value: &[u8]) {
        self.buf[SIGNATURE_RANGE].copy_from_slice(value);
    }

    pub fn previous(&self) -> Name<30> {
        Name::from(&self.buf[BLOCK_PREVIOUS_RANGE])
    }

    pub fn set_previous(&mut self, value: &[u8]) {
        self.buf[BLOCK_PREVIOUS_RANGE].copy_from_slice(value);
    }

    pub fn payload(&self) -> Name<30> {
        Name::from(&self.buf[BLOCK_PAYLOAD_RANGE])
    }

    pub fn set_payload(&mut self, value: &[u8]) {
        self.buf[BLOCK_PAYLOAD_RANGE].copy_from_slice(value);
    }
}


pub struct Chain {
    pk: sign::PublicKey,
    header: Header,
    block: Block,
    file: fs::File,
}

impl Chain {
    pub fn verify_chain(&mut self) -> io::Result<bool> {
        let mut br = io::BufReader::new(self.file.try_clone()?);
        br.seek(io::SeekFrom::Start(0))?;
        br.read_exact(self.header.as_mut_buf())?;
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
    fn test_header_get_set() {
        let mut header = Header::new();

        let mut hash: Name<30> = Name::new();
        assert_eq!(header.hash(), hash);
        hash.randomize();
        assert_ne!(header.hash(), hash);
        header.set_hash(&hash);
        assert_eq!(header.hash(), hash);

        assert_eq!(header.signature().unwrap().as_ref(), [0; 64]);
        let (pk, sk) = sign::gen_keypair();
        let sig = sign::sign_detached(pk.as_ref(), &sk);
        assert_ne!(header.signature().unwrap(), sig);
        header.set_signature(&sig);
        assert_eq!(header.signature().unwrap(), sig);

        assert_eq!(header.pubkey().as_ref(), [0; 32]);
        assert_ne!(header.pubkey(), pk);
        header.set_pubkey(&pk);
        assert_eq!(header.pubkey(), pk);
    }

    #[test]
    fn test_block_chain() {
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

