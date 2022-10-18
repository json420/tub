/*  FIXME: Skein probably provides better performance and a better security
    margin than Blake2b, so we should strongly consider Skein.
*/
use blake2::{Blake2b, Digest};
use digest::consts::{U30};
use generic_array::GenericArray;

type Blake2b240 = Blake2b<U30>;


pub fn hash(buf: &[u8]) -> GenericArray<u8, U30> {
    let mut h = Blake2b240::new();
    h.update(buf);
    h.finalize()
}


#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use super::*;

    static D1: &[u8] = b"my_input";
    static D1H240: [u8; 30] = hex!("35f6b8fe184790c47717de56324629309370b1f37b1be1736027d414c122");

    #[test]
    fn test_hash() {
        let mut h = Blake2b240::new();
        h.update(D1);
        let res = h.finalize();
        assert_eq!(res[..], (D1H240[..])[..]);

        let res = hash(D1);
        assert_eq!(res[..], D1H240[..]);
    }
}
