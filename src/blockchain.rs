//! Super cool crypto in here, yo! 💵 💵 💵

use std::{fs, io};
use std::os::unix::fs::FileExt;
use std::io::prelude::*;
use sodiumoxide::crypto::sign;
use blake3;
use crate::chaos::Name;


/*
The idea is to use a signed block chain in which the key rotates each signature
(given key is only used once).  The key is used for short term access control,
whereas the hash chain provides long term history preservation (even if the
key is compromised).

You can think of a Tub blockchain like a namespace or an identity.

ROOT, SIGNATURE, PUBKEY, PREVIOUS, COUNTER, TIMESTAMP, PAYLOAD_HASH




*/


// FIXME: lame, just to get things going
fn compute_hash(payload: &[u8]) -> Name<30> {
    let mut h = blake3::Hasher::new();
    h.update(payload);
    let mut hash = Name::new();
    h.finalize_xof().fill(hash.as_mut_buf());
    hash
}


pub type Secret = [u8; 64];



// SIG PREVIOUS PAYLOAD
pub struct WriteBlock<'a, const N: usize> {
    inner: &'a mut [u8],
}

impl<'a, const N: usize> WriteBlock<'a, N> {
    pub fn new(inner: &'a mut [u8]) -> Self{
        Self {inner: inner}
    }

    pub fn sign(&mut self, sk: &sign::SecretKey) -> sign::Signature {
        let sig = sign::sign_detached(self.as_signed(), &sk);
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


pub struct ReadBlock<'a, const N: usize> {
    inner: &'a [u8],
}

impl<'a, const N: usize> ReadBlock<'a, N> {
    pub fn new(inner: &'a [u8]) -> Self{
        Self {inner: inner}
    }

    pub fn as_signature(&self) -> &[u8] {
        &self.inner[0..64]
    }

    pub fn as_previous(&self) -> &[u8] {
        &self.inner[64..64 + N]
    }

    pub fn as_payload(&self) -> &[u8] {
        &self.inner[64 + N..64 + N * 2]
    }
}




// HASH SIG PUBKEY

pub struct Header {
    buf: [u8; 30 + 64 + 32],
}

impl Header {
    pub fn new() -> Self {
        Self {buf: [0; 30 + 64 + 32]}
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_tail(&self) -> &[u8] {
        &self.buf[30..]
    }

    pub fn as_hash(&self) -> &[u8] {
        &self.buf[0..30]
    }

    pub fn as_pubkey(&self) -> &[u8] {
        &self.buf[94..126]
    }

    pub fn set_hash(&mut self, value: &[u8]) {
        self.buf[0..30].copy_from_slice(value);
    }

    pub fn set_signature(&mut self, value: &[u8]) {
        self.buf[30..94].copy_from_slice(value);
    }

    pub fn set_pubkey(&mut self, value: &[u8]) {
        self.buf[94..126].copy_from_slice(value);
    }

    pub fn hash(&self) -> Name<30> {
        Name::from(self.as_hash())
    }

    pub fn pubkey(&self) -> sign::PublicKey {
        sign::PublicKey::from_slice(self.as_pubkey()).unwrap()
    }

    pub fn compute(&self) -> Name<30> {
        compute_hash(self.as_tail())
    }

    pub fn sign_and_set(&mut self, sk: &sign::SecretKey) -> Name<30> {
        let pk = sk.public_key();
        let sig = sign::sign_detached(pk.as_ref(), &sk);
        self.set_pubkey(pk.as_ref());
        self.set_signature(sig.as_ref());
        let hash = self.compute();
        self.set_hash(hash.as_buf());
        hash
    }

    pub fn is_valid(&self) -> bool {
        let pk = self.pubkey();
        if let Ok(_) = sign::verify(self.as_tail(), &pk) {
            self.hash() == self.compute()
        }
        else {
            false
        }
    }
}


//          This gets signed
//          ******************
// HASH SIG PAYLOAD [PREVIOUS]  <-- [PREV] is used but not stored in block
//      ^^^^^^^^^^^^^^^^^^^^^^
//      This get's hashed

pub struct Block {
    pk: sign::PublicKey,
    buf: [u8; 154],
}

impl Block {
    pub fn new(pk: sign::PublicKey) -> Self {
        Self {pk: pk, buf: [0; 154]}
    }

    pub fn hash(&self) -> Name<30> {
        Name::from(&self.buf[0..30])
    }

    pub fn previous(&self) -> Name<30> {
        Name::from(&self.buf[124..154])
    }

    pub fn as_mut_raw(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        self.ready_next();
        &mut self.buf[0..124]  // Everything except PREVIOUS
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf[0..124]  // Everything except PREVIOUS
    }

    pub fn as_signed(&self) -> &[u8] {
        &self.buf[94..]
    }

    pub fn as_hashed(&self) -> &[u8] {
        &self.buf[30..]
    }

    pub fn set_hash(&mut self, value: &[u8]) {
        self.buf[0..30].copy_from_slice(value);
    }

    pub fn set_signature(&mut self, value: &[u8]) {
        self.buf[30..94].copy_from_slice(value);
    }

    pub fn set_payload(&mut self, value: &[u8]) {
        self.buf[94..124].copy_from_slice(value);
    }

    pub fn set_previous(&mut self, value: &[u8]) {
        self.buf[124..154].copy_from_slice(value);
    }

    pub fn ready_next(&mut self) {
        let hash = self.hash();
        self.set_previous(hash.as_buf());
        assert_eq!(hash, self.previous());
        self.buf[0..124].fill(0);
    }

    pub fn sign_next(&mut self, payload: &Name<30>, sk: &sign::SecretKey) -> Name<30> {
        self.ready_next();
        self.set_payload(payload.as_buf());
        let sig = sign::sign_detached(self.as_signed(), sk);
        self.set_signature(sig.as_ref());
        let hash = self.compute();
        self.set_hash(hash.as_buf());
        //assert!(self.is_valid());
        hash
    }

    pub fn compute(&self) -> Name<30> {
        compute_hash(self.as_hashed())
    }

    pub fn is_valid(&self) -> bool {
        if let Ok(_) = sign::verify(self.as_hashed(), &self.pk) {
            self.hash() == self.compute()
        }
        else {
            false
        }
    }
}


/*

1. Generate SecretKey
2. Generate header
3. Write header to file
4. Reopen file?



*/

pub struct Chain {
    file: fs::File,
    pub header: Header,
    pub block: Block,
}

impl Chain {
    pub fn new(file: fs::File, pk: sign::PublicKey) -> Self {
        Self {
            file: file,
            header: Header::new(),
            block: Block::new(pk),

        }
    }

    pub fn generate(file: fs::File) -> (sign::SecretKey, Self) {
        let (pk, sk) = sign::gen_keypair();
        let mut chain = Self::new(file, pk);
        chain.init(&sk);
        (sk, chain)
    }

    fn init(&mut self, sk: &sign::SecretKey) {
        let hash = self.header.sign_and_set(sk);
        self.block.set_hash(hash.as_buf());
        self.file.write_all(self.header.as_buf()).unwrap();
    }

    pub fn sign_next(&mut self, payload: &Name<30>, sk: &sign::SecretKey) -> io::Result<Name<30>> {
        let hash = self.block.sign_next(payload, sk);
        self.file.write_all(self.block.as_buf())?;
        Ok(hash)
    }

    pub fn verify(&mut self) -> io::Result<()> {
        self.file.seek(io::SeekFrom::Start(0))?;
        let mut br = io::BufReader::new(self.file.try_clone()?);
        if let Ok(_) = br.read_exact(&mut self.header.as_mut_buf()) {
            if ! self.header.is_valid() {
                panic!("Bad chain header, yo");
            }
            self.block.set_hash(self.header.hash().as_buf());
            while let Ok(_) = br.read_exact(self.block.as_mut_buf()) {
                if ! self.block.is_valid() {
                    panic!("Bad block, yo");
                }
            }
        }
        Ok(())
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
        let mut writer: WriteBlock<30> = WriteBlock::new(obj.as_mut_data());
        let mut name = DefaultName::new();
        name.randomize();
        writer.set_previous(name.as_buf());
        name.randomize();
        writer.set_payload(name.as_buf());
        let (pk, sk) = sign::gen_keypair();
        let sig = writer.sign(&sk);
        assert!(sign::verify(obj.as_data(), &pk).is_ok());
    }

    #[test]
    fn test_header() {
        let mut header = Header::new();
        getrandom(header.as_mut_buf());
        assert!(! header.is_valid());

        let (pk, sk) = sign::gen_keypair();
        let hash = header.sign_and_set(&sk);
        assert_eq!(hash, header.hash());
        assert_eq!(hash, header.compute());
        assert_eq!(pk.as_ref(), header.as_pubkey());
        assert!(header.is_valid());
        for bit in 0..header.as_mut_buf().len() * 8 {
            flip_bit_in(header.as_mut_buf(), bit);
            assert!(! header.is_valid());
            flip_bit_in(header.as_mut_buf(), bit);
            assert!(header.is_valid());
        }
    }

    #[test]
    fn test_block() {
        let (pk, sk) = sign::gen_keypair();
        let mut block = Block::new(pk);
        let mut payload: Name<30> = Name::new();
        payload.randomize();
        let mut previous: Name<30> = Name::new();
        previous.randomize();
        assert!(! block.is_valid());
        block.sign_next(&payload, &sk);
        assert!(block.is_valid());
        for bit in 0..block.as_mut_raw().len() * 8 {
            flip_bit_in(block.as_mut_raw(), bit);
            assert!(! block.is_valid());
            flip_bit_in(block.as_mut_raw(), bit);
            assert!(block.is_valid());
        }
        for _ in 0..100 {
            payload.randomize();
            let prev = block.hash();
            let new = block.sign_next(&payload, &sk);
            assert_eq!(block.previous(), prev);
        }
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

