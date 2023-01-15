//! Super cool crypto in here, yo! 💵 💵 💵

use sodiumoxide::crypto::sign;
use blake3;


/*
The idea is to use a signed block chain in which the key rotates each signature
(given key is only used once).  The key is used for short term access control,
whereas the hash chain provides long term history preservation (even if the
key is compromised).

You can think of a Tub blockchain like a namespace or an identity.

ROOT, SIGNATURE, PUBKEY, PREVIOUS, COUNTER, TIMESTAMP, PAYLOAD_HASH


*/

pub type Secret = [u8; 64];


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
        let mut key = [0_u8; 64];
        h.finalize_xof().fill(&mut key);
        key
        //sign::SecretKey::from_slice(&key).unwrap()
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use crate::util::getrandom;
    use super::*;

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

