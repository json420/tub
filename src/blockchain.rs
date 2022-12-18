use sodiumoxide::crypto::sign;
use blake3;
use crate::base::*;



pub fn hash_block(data: &[u8]) -> TubHash {
    //assert!(data.len() > 0);
    let mut h = blake3::Hasher::new();
    h.update(b"Tub/block");  // <-- FIXME: Do more better than this
    h.update(data);
    let mut hash: TubHash = [0_u8; TUB_HASH_LEN];
    h.finalize_xof().fill(&mut hash);
    hash
}


/*

ROOT, SIGNATURE, PUBKEY, PREVIOUS, COUNTER, TIMESTAMP, PAYLOAD_HASH

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

pub struct BlockChain {
    pk: sign::PublicKey,
    sk: sign::SecretKey,
    cnt: u64,
    current_hash: TubHash,
    buf: [u8; BLOCK_LEN],
}

impl BlockChain {
    pub fn generate() -> Self {
        let (pk, sk) = sign::gen_keypair();
        Self {
            pk: pk,
            sk: sk,
            cnt: 0,
            current_hash: [0_u8; TUB_HASH_LEN],
            buf: [0_u8; BLOCK_LEN],
        }
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn as_signable(&self) -> &[u8] {
        &self.buf[BLOCK_SIGNABLE_RANGE]
    }

    pub fn as_signature(&self) -> &[u8] {
        &self.buf[BLOCK_SIGNATURE_RANGE]
    }

    fn set_pubkey(&mut self, hash: &TubHash) {
        self.buf[BLOCK_PREVIOUS_RANGE].copy_from_slice(hash);
    }

    pub fn as_pubkey(&self) -> &[u8] {
        &self.buf[BLOCK_PUBKEY_RANGE]
    }

    fn set_previous(&mut self, hash: &TubHash) {
        self.buf[BLOCK_PREVIOUS_RANGE].copy_from_slice(hash);
    }

    pub fn as_previous(&self) -> &[u8] {
        &self.buf[BLOCK_PREVIOUS_RANGE]
    }

    fn set_block_type(&mut self, kind: ObjectType) {
        self.buf[BLOCK_TYPE_INDEX] = kind as u8;
    }

    pub fn block_type(&self) -> BlockType {
        self.buf[BLOCK_TYPE_INDEX].into()
    }

    fn set_counter(&mut self, counter: u64) {
        self.buf[BLOCK_COUNTER_RANGE].copy_from_slice(&counter.to_le_bytes());
    }

    pub fn counter(&self) -> u64 {
        u64::from_le_bytes(
            self.buf[BLOCK_COUNTER_RANGE].try_into().expect("oops")
        )
    }

    fn set_timestamp(&mut self, timestamp: u64) {
        self.buf[BLOCK_TIMESTAMP_RANGE].copy_from_slice(&timestamp.to_le_bytes());
    }

    pub fn timestamp(&self) -> u64 {
        u64::from_le_bytes(
            self.buf[BLOCK_TIMESTAMP_RANGE].try_into().expect("oops")
        )
    }

    fn set_payload(&mut self, hash: &TubHash) {
        self.buf[BLOCK_PAYLOAD_RANGE].copy_from_slice(hash);
    }

    pub fn as_payload(&self) -> &[u8] {
        &self.buf[BLOCK_PAYLOAD_RANGE]
    }

    pub fn append(&mut self, kind: BlockType, payload_hash: &TubHash) {
        let cur = self.current_hash;
        self.set_previous(&cur);
        self.set_counter(self.cnt);
        self.cnt += 1; 

        // self.set_timestamp() FIXME
        self.set_payload(payload_hash);

        let sig = sign::sign_detached(self.as_signable(), &self.sk);
        assert!(sign::verify_detached(&sig, self.as_signable(), &self.pk));
        self.buf[BLOCK_SIGNATURE_RANGE].copy_from_slice(&sig.to_bytes());

        self.current_hash = hash_block(self.as_buf());
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
