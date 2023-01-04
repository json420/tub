use seahash;
use crate::base::*;
use crate::protocol::{Hasher, TubName, Blake3};
/*

Generic object format:


| HASH | SIZE | TYPE| PAYLOAD |



*/


pub struct Info {
    val: u32,
}

impl Info {
    fn new(size: usize, kind: u8) -> Self {
        assert!(size > 0);
        assert!(size <= 16777216);
        Self {val: (size - 1) as u32 | (kind as u32) << 24}
    }

    pub fn from_le_bytes(buf: &[u8]) -> Self {
        Self {val: u32::from_le_bytes(buf.try_into().expect("oops"))}
    }

    pub fn to_le_bytes(&self) -> [u8; 4] {
        self.val.to_le_bytes()
    }

    pub fn raw(&self) -> u32 {
        self.val
    }

    fn size(&self) -> usize {
        ((self.val & 0x00ffffff) + 1) as usize
    }

    fn kind(&self) -> u8 {
        (self.val >> 24) as u8
    }
}


pub struct Object<const N: usize, H: Hasher> {
    buf: Vec<u8>,
    hasher: H,
}

impl<const N: usize, H: Hasher> Object<N, H> {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            hasher: H::new(),
        }
    }

    pub fn reset(&mut self) {
        self.buf.clear();
        self.buf.resize(N + 4, 0);
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn compute(&self) -> TubName<N> {
        let mut hash: TubName<N> = TubName::new();
        self.hasher.hash_into(
            self.info().raw(), self.as_data(),
            hash.as_mut_buf()
        );
        hash
    }

    pub fn finalize(&mut self) -> TubName<N> {
        assert_eq!(self.buf.len(), OBJECT_HEADER_LEN + self.info().size());
        let hash = self.compute();
        self.buf[0..N].copy_from_slice(hash.as_buf());
        hash
    }

    pub fn info(&self) -> Info {
        Info::from_le_bytes(&self.buf[N..N + 4])
    }

    pub fn set_info(&mut self, info: Info) {
        self.buf[N..N + 4].copy_from_slice(&info.to_le_bytes());
    }

    pub fn resize_to_info(&mut self) {
        self.buf.resize(N + 4 + self.info().size(), 0);
    }

    pub fn as_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_mut_buf(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn as_mut_vec(&mut self) -> &Vec<u8> {
        &mut self.buf
    }

    pub fn as_mut_header(&mut self) -> &mut [u8] {
        &mut self.buf[0..N + 4]
    }

    pub fn as_data(&self) -> &[u8] {
        &self.buf[N + 4..]
    }

    pub fn as_mut_data(&mut self) -> &mut [u8] {
        &mut self.buf[N + 4..]
    }
}


/*

pub struct Store<const N: usize, P: Protocol> {
    protocol: P,
}


impl<const N: usize, P: Protocol> Store<N, P> {
    pub fn load(&self, hash: TubHash, obj: &mut Object<N, P>) -> bool {
        true
    }

}
*/

struct Junk {
    buf: [u8; 30],
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ProtocolZero;

    #[test]
    fn test_info() {
        let info = Info::from_le_bytes(&[0; 4]);
        assert_eq!(info.size(), 1);
        assert_eq!(info.kind(), 0);

        let info = Info::new(1, 0);
        assert_eq!(info.raw(), 0);
        assert_eq!(info.size(), 1);
    }

    #[test]
    #[should_panic(expected="")]
    fn test_info_assert() {
        let sk = Info::new(0, 0);
    }

    #[test]
    fn test_object() {
        let mut obj: Object<30, Blake3> = Object::new();
        assert_eq!(obj.len(), 0);
        obj.reset();
        assert_eq!(obj.len(), 34);

        assert_eq!(obj.info().size(), 1);
        assert_eq!(obj.info().kind(), 0);
        assert_eq!(obj.as_buf(), &[0; 34]);

        obj.as_mut_buf().fill(255);
        assert_eq!(obj.info().size(), 16 * 1024 * 1024);
        assert_eq!(obj.info().kind(), 255);

        assert_eq!(obj.len(), 34);
        assert_eq!(obj.as_buf(), &[255; 34]);

        obj.reset();
        assert_eq!(obj.len(), 34);
        assert_eq!(obj.as_buf(), &[0; 34]);
    }

    #[test]
    fn test_seahash() {
        let buf = [42; 69];
        let r = seahash::hash(&buf);
    }
}

