use sodiumoxide::crypto::sign;
use std::io::prelude::*;
use std::os::unix::fs::FileExt;
use std::fs;
use std::io;
use blake3;
use crate::base::*;



pub fn hash_block(data: &[u8]) -> TubHash {
    assert!(data.len() > 0);
    let mut h = blake3::Hasher::new();
    h.update(b"Tub/block");  // <-- FIXME: Do more better than this
    h.update(data);
    let mut hash: TubHash = [0_u8; TUB_HASH_LEN];
    h.finalize_xof().fill(&mut hash);
    hash
}


/*

ROOT, PREVIOUS, PAYLOAD_HASH

*/

pub struct Block {
    buf: [u8; BLOCK_LEN],
}

impl Block
{
    pub fn new() ->  Self {
        Self {buf: [0_u8; BLOCK_LEN]}
    }

    pub fn reset(&mut self) {
        self.buf.fill(0);
    }

    pub fn is_valid(&self) -> bool {
        self.as_hash() == self.compute()
    }

    fn compute(&self) -> TubHash {
        hash_block(self.as_tail())
    }

    pub fn as_hash(&self) -> &[u8] {
        &self.buf[0..TUB_HASH_LEN]
    }

    fn set_hash(&mut self, hash: &TubHash) {
        self.buf[0..TUB_HASH_LEN].copy_from_slice(hash);
    }

    pub fn set_payload_hash(&mut self, hash: &TubHash) {
        self.buf[BLOCK_PAYLOAD_HASH_RANGE].copy_from_slice(hash);
    }

    pub fn hash(&self) -> TubHash {
        self.as_hash().try_into().expect("oops")
    }

    pub fn as_tail(&self) -> &[u8] {
        &self.buf[TUB_HASH_LEN..]
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }
}


pub struct Chain {
    file: fs::File,
    index: u64,
}

impl Chain {
    pub fn new(file: fs::File) -> Self {
        Self {file: file, index: 0}
    }

    pub fn add_block(&mut self, refhash: &TubHash) -> TubHash {
        [0_u8; TUB_HASH_LEN]
    }

    pub fn read_next_block(&mut self, block: &mut Block) -> io::Result<()> {
        let offset = self.index * BLOCK_LEN as u64;
        self.index += 1;
        self.file.read_exact_at(block.as_mut_buf(), offset)?;
        Ok(())
    }

    pub fn write_next_block(&mut self, block: &Block) -> io::Result<()> {
        self.file.write_all(block.as_buf());
        self.index += 1;
        Ok(())
    }

    pub fn verify_chain(&mut self) -> io::Result<bool> {
        self.index = 0;
        let mut block = Block::new();
        while let Ok(_) = self.read_next_block(&mut block) {
        }
        Ok(true)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::getrandom;

    #[test]
    fn test_block() {
        let mut b = Block::new();
        assert!(! b.is_valid());
        assert_eq!(b.compute(), hash_block(&[0_u8; BLOCK_LEN - TUB_HASH_LEN]));
        assert_eq!(b.as_hash(), [0_u8; TUB_HASH_LEN]);
        assert_eq!(b.hash(), [0_u8; TUB_HASH_LEN]);
        assert_eq!(b.as_tail(), [0_u8; BLOCK_LEN - TUB_HASH_LEN]);
        assert_eq!(b.as_mut_buf(), [0_u8; BLOCK_LEN]);

        getrandom(b.as_mut_buf());
        assert!(! b.is_valid());
        let value = b.compute();
        assert_eq!(value, hash_block(b.as_tail()));
        assert_ne!(b.as_hash(), value);
        b.set_hash(&value);
        assert!(b.is_valid());
        assert_eq!(b.as_hash(), value);
        assert_eq!(b.hash(), value);
    }

    #[test]
    fn test_stuff() {
        let (pk, sk) = sign::gen_keypair();
        let data = b"some data";
        let signed_data = sign::sign(data, &sk);
        let verified_data = sign::verify(&signed_data, &pk).unwrap();
        assert_eq!(data, &verified_data[..]);
    }
}
