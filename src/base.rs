//! Constants and types.

pub const INFO_LEN: usize = 4;
pub const OBJECT_MAX_SIZE: usize = 16777216;

pub const DOTDIR: &str = ".tub";
pub const DOTIGNORE: &str = ".tubignore";
pub const PACKFILE: &str = "append.tub";
pub const INDEX_FILE: &str = "append.idx";
pub const OBJECTDIR: &str = "objects";
pub const TMPDIR: &str = "tmp";
pub const README: &str = "REAMDE.txt";  // The REAMDE file
pub const BRANCHES: &str = "blockchain";

pub static README_CONTENTS: &[u8] = b"Hello from Bathtub DB!

What's even more relaxing than a Couch?  A Bathtub!
";


#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ObjectType {
    Data,
    Tree,
    Block,
    Commit,
}

impl From<u8> for ObjectType {
    fn from(item: u8) -> Self {
        match item {
            0 => Self::Data,
            1 => Self::Tree,
            2 => Self::Block,
            3 => Self::Commit,
            _ => panic!("Unknown ObjectType: {}", item),
        }
    }
}

#[derive(Debug)]
pub enum ObjKind {
    Invalid,
    Data,
    BigData,
    Key,
    Block,
    Stream,
    Tree,
    Commit,
    Fanout,
}

impl From<u8> for ObjKind {
    fn from(item: u8) -> Self {
        match item {
            0 => Self::Invalid,
            1 => Self::Key,
            2 => Self::Block,
            3 => Self::Data,
            4 => Self::BigData,
            5 => Self::Stream,
            6 => Self::Tree,
            7 => Self::Commit,
            8 => Self::Fanout,
            _ => panic!("Unknown ObjKind: {}", item),
        }
    }
}



#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BlockType {
    Configure,
    Commit,
}

impl From<u8> for BlockType {
    fn from(item: u8) -> Self {
        match item {
            0 => Self::Configure,
            1 => Self::Commit,
            _ => panic!("Unknown BlockType: {}", item),
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objtype() {
        for k in 0..3 {
            let ot: ObjectType = k.into();
            assert_eq!(ot as u8, k);
        }
        assert_eq!(ObjectType::Data as u8, 0);
        assert_eq!(ObjectType::Data, 0.into());
        assert_eq!(ObjectType::Tree as u8, 1);
        assert_eq!(ObjectType::Tree, 1.into());
        assert_eq!(ObjectType::Block as u8, 2);
        assert_eq!(ObjectType::Block, 2.into());
        assert_eq!(ObjectType::Commit as u8, 3);
        assert_eq!(ObjectType::Commit, 3.into());
    }

    #[test]
    #[should_panic(expected = "Unknown ObjectType: 4")]
    fn test_objtype_panic1() {
        let _kind: ObjectType = 4.into();
    }

    #[test]
    #[should_panic(expected = "Unknown ObjectType: 255")]
    fn test_objtype_panic2() {
        let _kind: ObjectType = 255.into();
    }

    #[test]
    fn test_blocktype() {
        for k in 0..2 {
            let ot: BlockType = k.into();
            assert_eq!(ot as u8, k);
        }
        assert_eq!(BlockType::Configure as u8, 0);
        assert_eq!(BlockType::Configure, 0.into());
        assert_eq!(BlockType::Commit as u8, 1);
        assert_eq!(BlockType::Commit, 1.into());
    }

    #[test]
    #[should_panic(expected = "Unknown BlockType: 2")]
    fn test_blockype_panic1() {
        let _kind: BlockType = 2.into();
    }

    #[test]
    #[should_panic(expected = "Unknown BlockType: 255")]
    fn test_block_type_panic2() {
        let _kind: BlockType = 255.into();
    }
}

