pub const ABSTRACT_ID_SIZE: usize = 15;
pub const OBJECT_ID_SIZE: usize = 30;

pub type AbstractID = [u8; ABSTRACT_ID_SIZE];
pub type ObjectID = [u8; OBJECT_ID_SIZE];

pub type ObjectSize = u64;



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sizes() {
        assert_eq!(ABSTRACT_ID_SIZE % 5, 0);
        assert_eq!(OBJECT_ID_SIZE % 5, 0);
        assert!(OBJECT_ID_SIZE > ABSTRACT_ID_SIZE);
    }
}

