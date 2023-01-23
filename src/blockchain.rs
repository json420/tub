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

    pub fn compute(&self) -> Name<30> {
        compute_hash(&self.buf[HEADER_HASHED_RANGE])
    }

    pub fn sign(&mut self, sk: &sign::SecretKey) -> sign::Signature {
        let pk = sk.public_key();
        let sig = sign::sign_detached(pk.as_ref(), sk);
        self.set_signature(&sig);
        self.set_pubkey(&pk);
        self.set_hash(&self.compute());
        sig
    }

    pub fn verify_hash(&self) -> bool {
        self.hash() == self.compute()
    }

    pub fn verify_signature(&self) -> bool {
        if let Ok(sig) = self.signature() {
            sign::verify_detached(&sig, &self.buf[HEADER_PUBKEY_RANGE], &self.pubkey())
        }
        else {
            false
        }
    }

    pub fn verify(&self) -> bool {
        self.verify_hash() && self.verify_signature()
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

    pub fn len(&self) -> usize {
        self.buf.len()
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
        self.set_signature(&sig);
        self.set_hash(&self.compute());
        sig
    }

    pub fn as_signed(&self) -> &[u8] {
        &self.buf[BLOCK_SIGNED_RANGE]
    }

    pub fn verify_hash(&self) -> bool {
        self.hash() == self.compute()
    }

    pub fn verify_signature(&self) -> bool {
        if let Ok(sig) = self.signature() {
            sign::verify_detached(&sig, self.as_signed(), &self.pk)
        }
        else {
            false
        }
    }

    pub fn verify(&self) -> bool {
        self.verify_hash() && self.verify_signature()
    }

    pub fn verify_against(&self, previous: &Name<30>) -> bool {
        self.verify() && &self.previous() == previous
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

    pub fn previous(&self) -> Name<30> {
        Name::from(&self.buf[BLOCK_PREVIOUS_RANGE])
    }

    pub fn set_previous(&mut self, hash: &Name<30>) {
        self.buf[BLOCK_PREVIOUS_RANGE].copy_from_slice(hash.as_buf());
    }

    pub fn payload(&self) -> Name<30> {
        Name::from(&self.buf[BLOCK_PAYLOAD_RANGE])
    }

    pub fn set_payload(&mut self, hash: &Name<30>) {
        self.buf[BLOCK_PAYLOAD_RANGE].copy_from_slice(hash.as_buf());
    }
}


pub struct Chain {
    pub header: Header,
    pub block: Block,
    file: fs::File,
}

impl Chain {
    pub fn generate(file: fs::File) -> io::Result<(sign::SecretKey, Self)> {
        let (pk, sk) = sign::gen_keypair();
        let me = Self::create(file, &sk)?;
        assert_eq!(me.header.pubkey(), pk);
        Ok((sk, me))
    }

    pub fn create(mut file: fs::File, sk: &sign::SecretKey) -> io::Result<Self> {
        let mut header = Header::new();
        header.sign(sk);
        file.write_all(header.as_buf())?;
        let mut block = Block::new(header.pubkey());
        block.set_hash(&header.hash());
        Ok( Self {
            header: header,
            block: block,
            file: file,
        })
    }

    pub fn sign_next(&mut self, payload: &Name<30>, sk: &sign::SecretKey) -> io::Result<()> {
        self.block.set_payload(payload);
        self.block.set_previous(&self.block.hash());
        self.block.sign(sk);
        self.file.write_all(self.block.as_buf())?;
        Ok(())
    }

    pub fn verify(&mut self) -> io::Result<bool> {
        let mut br = io::BufReader::new(self.file.try_clone()?);
        br.seek(io::SeekFrom::Start(0))?;
        br.read_exact(self.header.as_mut_buf())?;
        if ! self.header.verify() {
            panic!("Bad header: {}", self.header.hash());
        }
        let mut previous = self.header.hash();
        while let Ok(_) = self.file.read_exact(self.block.as_mut_buf()) {
            if ! self.block.verify_against(&previous) {
                panic!("Bad block: {}", self.block.hash());
            }
            previous = self.block.hash();
        }
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
    fn test_header_verify_hash() {
        let mut header = Header::new();
        assert!(! header.verify_hash());
        getrandom(header.as_mut_buf());
        assert!(! header.verify_hash());
        header.set_hash(&header.compute());
        assert!(header.verify_hash());
        let count = header.as_mut_buf().len() * 8;
        for bit in 0..count {
            flip_bit_in(header.as_mut_buf(), bit);
            assert!(! header.verify_hash());
            flip_bit_in(header.as_mut_buf(), bit);
            assert!(header.verify_hash());
        }
    }

    #[test]
    fn test_header_verify_signature() {
        let mut header = Header::new();
        assert!(! header.verify_signature());

        let (pk, sk) = sign::gen_keypair();
        header.sign(&sk);
        assert!(header.verify_signature());

        let start = 30 * 8;
        let stop = header.as_mut_buf().len() * 8;
        for bit in 0..start {
            flip_bit_in(header.as_mut_buf(), bit);
            assert!(header.verify_signature());
        }
        for bit in start..stop {
            flip_bit_in(header.as_mut_buf(), bit);
            assert!(! header.verify_signature());
            flip_bit_in(header.as_mut_buf(), bit);
            assert!(header.verify_signature());
        }
    }

    #[test]
    fn test_block_get_set() {
        let (pk, sk) = sign::gen_keypair();
        let mut block = Block::new(pk);

        let mut hash: Name<30> = Name::new();
        assert_eq!(block.hash(), hash);
        hash.randomize();
        assert_ne!(block.hash(), hash);
        block.set_hash(&hash);
        assert_eq!(block.hash(), hash);

        assert_eq!(block.signature().unwrap().as_ref(), [0; 64]);
        let sig = sign::sign_detached(pk.as_ref(), &sk);
        assert_ne!(block.signature().unwrap(), sig);
        block.set_signature(&sig);
        assert_eq!(block.signature().unwrap(), sig);

        assert_eq!(block.previous(), Name::new());
        hash.randomize();
        assert_ne!(block.previous(), hash);
        block.set_previous(&hash);
        assert_eq!(block.previous(), hash);

        assert_eq!(block.payload(), Name::new());
        hash.randomize();
        assert_ne!(block.payload(), hash);
        block.set_payload(&hash);
        assert_eq!(block.payload(), hash);
    }

    #[test]
    fn test_block_verify_hash() {
        let mut block = Header::new();
        assert!(! block.verify_hash());
        getrandom(block.as_mut_buf());
        assert!(! block.verify_hash());
        block.set_hash(&block.compute());
        assert!(block.verify_hash());
        let count = block.as_mut_buf().len() * 8;
        for bit in 0..count {
            flip_bit_in(block.as_mut_buf(), bit);
            assert!(! block.verify_hash());
            flip_bit_in(block.as_mut_buf(), bit);
            assert!(block.verify_hash());
        }
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

