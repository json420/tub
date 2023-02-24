//! Super cool crypto in here, yo! 💵 💵 💵

use std::{io, fs};
use std::ops::Range;
use std::io::prelude::*;
use std::os::unix::fs::FileExt;
use rand::rngs::OsRng;
use ed25519_dalek::{
    SigningKey,
    Signer,
    Signature,
    SignatureError,
    VerifyingKey,
    Verifier,
};
use blake3;
use crate::chaos::DefaultName;


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


fn compute_hash(payload: &[u8]) -> DefaultName {
    let mut h = blake3::Hasher::new();
    h.update(payload);
    let mut hash= DefaultName::new();
    h.finalize_xof().fill(hash.as_mut_buf());
    hash
}


const HASH_RANGE: Range<usize> = 0..30;
const SIGNATURE_RANGE: Range<usize> = 30..94;

const HEADER_LEN: usize = 126;
const HEADER_PUBKEY_RANGE: Range<usize> = 94..126;
const HEADER_HASHED_RANGE: Range<usize> = 30..126;

// HASH SIG PUBKEY
#[derive(Debug)]
pub struct Header {
    buf: [u8; HEADER_LEN],
}

impl Header {
    pub fn new() -> Self {
        Self {buf: [0; HEADER_LEN]}
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn compute(&self) -> DefaultName {
        compute_hash(&self.buf[HEADER_HASHED_RANGE])
    }

    pub fn sign(&mut self, sk: &SigningKey) -> Signature {
        let vk = sk.verifying_key();
        let sig = sk.sign(vk.as_bytes());
        self.set_signature(&sig);
        self.set_pubkey(&vk);
        self.set_hash(&self.compute());
        sig
    }

    pub fn verify_hash(&self) -> bool {
        self.hash() == self.compute()
    }

    pub fn verify_signature(&self) -> bool {
        if let Ok(sig) = self.signature() {
            self.pubkey().verify(&self.buf[HEADER_PUBKEY_RANGE], &sig).is_ok()
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

    pub fn hash(&self) -> DefaultName {
        DefaultName::from(&self.buf[HASH_RANGE])
    }

    pub fn set_hash(&mut self, hash: &DefaultName) {
        self.buf[HASH_RANGE].copy_from_slice(hash.as_buf());
    }

    // Note not all signature values are structurally valid
    pub fn signature(&self) -> Result<Signature, SignatureError> {
        Signature::try_from(&self.buf[SIGNATURE_RANGE])
    }

    pub fn set_signature(&mut self, sig: &Signature) {
        self.buf[SIGNATURE_RANGE].copy_from_slice(&sig.to_bytes());
    }

    pub fn pubkey(&self) -> VerifyingKey {
        let b: [u8; 32] = self.buf[HEADER_PUBKEY_RANGE].try_into().unwrap();
        VerifyingKey::from_bytes(&b).unwrap()
    }

    pub fn set_pubkey(&mut self, pk: &VerifyingKey) {
        self.buf[HEADER_PUBKEY_RANGE].copy_from_slice(pk.as_ref());
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}



const BLOCK_LEN: usize = 162;
const BLOCK_PREVIOUS_RANGE: Range<usize> = 94..124;
const BLOCK_PAYLOAD_RANGE: Range<usize> = 124..154;
const BLOCK_INDEX_RANGE: Range<usize> = 154..162;
const BLOCK_SIGNED_RANGE: Range<usize> = 94..154;
const BLOCK_HASHED_RANGE: Range<usize> = 30..154;

// 30    64     30       30
// HASH  SIG    PREVIOUS PAYLOAD
// 0..30 30..94 94..124  124..154
pub struct Block {
    buf: [u8; BLOCK_LEN],
    pk: VerifyingKey,
}

impl Block {
    pub fn new(pk: VerifyingKey) -> Self{
        Self {buf: [0; BLOCK_LEN], pk}
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn compute(&self) -> DefaultName {
        compute_hash(&self.buf[BLOCK_HASHED_RANGE])
    }

    pub fn sign(&mut self, sk: &SigningKey) -> Signature {
        let sig = sk.sign(self.as_signed());
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
            self.pk.verify(self.as_signed(), &sig).is_ok()
        }
        else {
            false
        }
    }

    pub fn verify(&self) -> bool {
        self.verify_hash() && self.verify_signature()
    }

    pub fn verify_against(&self, previous: &DefaultName) -> bool {
        self.verify() && &self.previous() == previous
    }

    pub fn hash(&self) -> DefaultName {
        DefaultName::from(&self.buf[HASH_RANGE])
    }

    pub fn set_hash(&mut self, hash: &DefaultName) {
        self.buf[HASH_RANGE].copy_from_slice(hash.as_buf());
    }

    // Note not all signature values are structurally valid
    pub fn signature(&self) -> Result<Signature, SignatureError> {
        Signature::try_from(&self.buf[SIGNATURE_RANGE])
    }

    pub fn set_signature(&mut self, sig: &Signature) {
        self.buf[SIGNATURE_RANGE].copy_from_slice(&sig.to_bytes());
    }

    pub fn previous(&self) -> DefaultName {
        DefaultName::from(&self.buf[BLOCK_PREVIOUS_RANGE])
    }

    pub fn set_previous(&mut self, hash: &DefaultName) {
        self.buf[BLOCK_PREVIOUS_RANGE].copy_from_slice(hash.as_buf());
    }

    pub fn payload(&self) -> DefaultName {
        DefaultName::from(&self.buf[BLOCK_PAYLOAD_RANGE])
    }

    pub fn set_payload(&mut self, hash: &DefaultName) {
        self.buf[BLOCK_PAYLOAD_RANGE].copy_from_slice(hash.as_buf());
    }

    pub fn index(&self) -> u64 {
        u64::from_le_bytes(
            self.buf[BLOCK_INDEX_RANGE].try_into().unwrap()
        )
    }

    pub fn set_index(&mut self, index: u64) {
        self.buf[BLOCK_INDEX_RANGE].copy_from_slice(&index.to_le_bytes());
    }
}


pub struct Chain {
    pub header: Header,
    pub block: Block,
    previous: DefaultName,
    file: fs::File,
    index: u64,
    current: u64,
    sk: Option<SigningKey>,
}

impl Chain {
    pub fn generate(file: fs::File) -> io::Result<Self> {
        let mut csprng = OsRng;
        let sk = SigningKey::generate(&mut csprng);
        Self::create(file, sk)
    }

    pub fn create(mut file: fs::File, sk: SigningKey) -> io::Result<Self> {
        let mut header = Header::new();
        header.sign(&sk);
        let previous = header.hash();
        file.write_all(header.as_buf())?;
        let block = Block::new(header.pubkey());
        Ok( Self {
            header,
            block,
            previous,
            file,
            index: 0,
            current: 0,
            sk: Some(sk),
        })
    }

    pub fn load_secret_key(&mut self, mut file: fs::File) -> io::Result<bool> {
        assert!(self.sk.is_none());
        let mut buf = [0_u8; 32];
        if file.read_exact(&mut buf).is_ok() {
            self.sk = Some(SigningKey::from_bytes(&buf));
        }
        Ok(self.sk.is_some())
    }

    pub fn save_secret_key(&self, mut file: fs::File) -> io::Result<()> {
        file.write_all(&self.sk.as_ref().unwrap().to_bytes())?;
        file.flush()
    }

    pub fn into_file(self) -> fs::File {
        self.file
    }

    pub fn open(mut file: fs::File) -> io::Result<Self> {
        file.seek(io::SeekFrom::Start(0))?;
        let mut header = Header::new();
        file.read_exact(header.as_mut_buf())?;
        let block = Block::new(header.pubkey());
        let previous = header.hash();
        let mut me = Self {
            header,
            block,
            previous,
            file,
            index: 0,
            current: 0,
            sk: None,
        };
        me.verify()?;
        Ok(me)
    }

    pub fn load_block_at(&mut self, index: u64) -> io::Result<bool> {
        let offset = self.header.len() as u64 + index * self.block.len() as u64;
        self.file.read_exact_at(self.block.as_mut_buf(), offset)?;
        Ok(self.block.verify())
    }

    pub fn seek_to_beyond(&mut self) {
        self.current = self.index;
    }

    pub fn load_previous(&mut self) -> io::Result<bool> {
        if self.current > 0 {
            self.current -= 1;
            self.load_block_at(self.current)
        }
        else {
            Ok(false)
        }
    }

    pub fn load_last_block(&mut self) -> io::Result<bool> {
        self.seek_to_beyond();
        self.load_previous()
    }

    pub fn sign_next(&mut self, payload: &DefaultName) -> io::Result<()> {
        self.block.set_payload(payload);
        self.block.set_previous(&self.previous);
        self.block.set_index(self.index);
        self.block.sign(self.sk.as_ref().unwrap());
        self.index += 1;
        self.previous = self.block.hash();
        self.file.write_all(self.block.as_buf())?;
        self.file.flush()?;
        Ok(())
    }

    pub fn verify(&mut self) -> io::Result<bool> {
        self.index = 0;
        let mut br = io::BufReader::new(self.file.try_clone()?);
        br.seek(io::SeekFrom::Start(0))?;
        br.read_exact(self.header.as_mut_buf())?;
        if ! self.header.verify() {
            panic!("Bad header: {}", self.header.hash());
        }
        self.previous = self.header.hash();
        while br.read_exact(self.block.as_mut_buf()).is_ok() {
            if ! self.block.verify_against(&self.previous) {
                panic!("Bad block: {} {}", self.block.hash(), &self.previous);
            }
            self.index += 1;
            self.previous = self.block.hash();
        }
        Ok(true)
    }
}


#[cfg(test)]
mod tests {
    use getrandom::getrandom;
    use crate::helpers::flip_bit_in;
    use crate::chaos::DefaultName;
    use super::*;
/*
    #[test]
    fn test_header_get_set() {
        let mut header = Header::new();

        let mut hash= DefaultName::new();
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

        let (_pk, sk) = sign::gen_keypair();
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

        let mut hash= DefaultName::new();
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

        assert_eq!(block.previous(), DefaultName::new());
        hash.randomize();
        assert_ne!(block.previous(), hash);
        block.set_previous(&hash);
        assert_eq!(block.previous(), hash);

        assert_eq!(block.payload(), DefaultName::new());
        hash.randomize();
        assert_ne!(block.payload(), hash);
        block.set_payload(&hash);
        assert_eq!(block.payload(), hash);

        assert_eq!(block.index(), 0);
        block.set_index(420);
        assert_eq!(block.index(), 420);
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
    fn test_ed25519_dalek() {
        let mut csprng = OsRng;
        let sk = SigningKey::generate(&mut csprng);
        let sig = sk.sign(b"hello");
        let pk = sk.verifying_key();
        assert!(pk.verify(b"hello", &sig).is_ok());
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

