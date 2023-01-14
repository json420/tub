//! Super cool crypto in here, yo!

use sodiumoxide::crypto::sign;


/*

ROOT, SIGNATURE, PUBKEY, PREVIOUS, COUNTER, TIMESTAMP, PAYLOAD_HASH

*/


#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::getrandom;

    #[test]
    fn test_ed25519() {
        let (pk, sk) = sign::gen_keypair();
        let data = b"some data";
        let signed_data = sign::sign(data, &sk);
        let verified_data = sign::verify(&signed_data, &pk).unwrap();
        assert_eq!(data, &verified_data[..]);
    }
}

