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


SIG PREVIOUS PAYLOAD

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


pub struct WriteBlock<'a, const N: usize> {
    inner: &'a mut [u8],
}

impl<'a, const N: usize> WriteBlock<'a, N> {
    pub fn new(inner: &'a mut [u8]) -> Self{
        Self {inner: inner}
    }

    pub fn as_mut_signature(&mut self) -> &mut [u8] {
        &mut self.inner[0..64]
    }

    pub fn as_mut_previous(&mut self) -> &mut [u8] {
        &mut self.inner[64..64 + N]
    }

    pub fn as_mut_payload(&mut self) -> &mut [u8] {
        &mut self.inner[64 + N..64 + N * 2]
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


pub struct Chain {
    secret: Secret,
}

impl Chain {
    pub fn resume(secret: Secret) -> Self {
        //let pk = sk.public_key();
        Self {secret: secret}
    }

    pub fn get_secret(&self, index: u64, previous: &[u8]) -> Secret {
        // FIXME: Do proper key derivation, we're just getting a feel...
        let mut h = blake3::Hasher::new();
        h.update(&self.secret);
        h.update(&index.to_le_bytes());
        h.update(previous);
        let mut key: Secret = [0_u8; 64];
        h.finalize_xof().fill(&mut key);
        key
        //sign::SecretKey::from_slice(&key).unwrap()
    }
}


/*


header: HASH SIG PUBKEY
block:  HASH SIG PAYLOAD
*/

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


pub struct BlockChain {
    file: fs::File,
    header: [u8; 30 + 64 + 32],
    block: [u8; 30 + 64 + 30],
}

impl BlockChain {
    pub fn new(file: fs::File) -> Self {
        Self {
            file: file,
            header: [0; 126],
            block: [0; 124],

        }
    }

    pub fn verify(&mut self) -> io::Result<()> {
        self.file.seek(io::SeekFrom::Start(0))?;
        let mut br = io::BufReader::new(self.file.try_clone()?);
        if let Ok(_) = br.read_exact(&mut self.header) {
            while let Ok(_) = br.read_exact(&mut self.block) {}
        }
        Ok(())
    }
}




#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use crate::util::getrandom;
    use super::*;

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
    }

    #[test]
    fn test_chain() {
        let mut secret = [0_u8; 64];
        getrandom(&mut secret);
        let chain = Chain::resume(secret);
        let mut set: HashSet<Secret> = HashSet::new();
        for i in 0..4269 {
            set.insert(chain.get_secret(i, b""));
        }
        assert_eq!(set.len(), 4269);
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

