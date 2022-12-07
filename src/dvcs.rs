//! Doodles on version control software built on Bathtub DB

use std::collections::HashMap;
use std::path::{PathBuf, Path};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::fs;
use std::io;
use std::convert::Into;

use crate::dbase32::db32enc_str;
use crate::store::Store;
use crate::base::*;


const MAX_DEPTH: usize = 32;


#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Kind {
    Dir,
    File,
    ExFile,
    Symlink,
}

impl From<u8> for Kind {
    fn from(item: u8) -> Self {
        match item {
            0 => Self::Dir,
            1 => Self::File,
            2 => Self::ExFile,
            3 => Self::Symlink,
            _ => panic!("Unknown Kind: {}", item),
        }
    }
}


#[derive(Debug, PartialEq)]
pub struct TreeEntry {
    kind: Kind,
    hash: TubHash,
}

impl TreeEntry {
    pub fn new(kind: Kind, hash: TubHash) -> Self {
        Self {kind: kind, hash: hash}
    }

    pub fn new_file(hash: TubHash) -> Self {
        Self {kind: Kind::File, hash: hash}
    }
}

pub type TreeMap = HashMap<PathBuf, TreeEntry>;



pub fn build_tree_state(dir: &Path, depth: usize) -> io::Result<()> {
    if depth < MAX_DEPTH {
        println!("Yo");
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let ft = entry.file_type()?;
            let path = entry.path();
            let name = path.file_name().unwrap();
            if name.to_str().unwrap().starts_with(".") {
                eprintln!("Skipping hiddin: {:?}", path);
            }
            else if ft.is_file() {
                println!("F {:?}", name);
            }
            else if ft.is_dir() {
                println!("D {:?}", name);
                build_tree_state(&path, depth + 1)?;
            }
        }
    }
    Ok(())
}


pub fn deserialize(buf: &[u8]) -> TreeMap {
    let mut map: TreeMap = HashMap::new();
    let mut offset = 0;
    while offset < buf.len() {
        let h: TubHash = buf[offset..offset + TUB_HASH_LEN].try_into().expect("oops");
        offset += h.len();

        let kind: Kind = buf[offset].into();
        let size = buf[offset + 1] as usize;
        offset += 2;

        let s = String::from_utf8(buf[offset..offset+size].to_vec()).unwrap();
        let pb = PathBuf::from(s);
        offset += size;

        map.insert(pb, TreeEntry::new(kind, h));
    }
    assert_eq!(offset, buf.len());
    map
}


pub fn serialize(map: &TreeMap) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut items = Vec::from_iter(map.iter());
    items.sort_by(|a, b| b.0.cmp(a.0));
    for (p, entry) in items.iter() {
        println!("{:?} {}", p, db32enc_str(&entry.hash));
        let path = p.to_str().unwrap().as_bytes();
        let size = path.len() as u8;
        buf.extend_from_slice(&entry.hash);
        buf.push(entry.kind as u8);
        buf.push(size);
        buf.extend_from_slice(path);
    }
    buf
}


pub struct Tree {
    map: TreeMap,
}


impl Tree {
    pub fn new() -> Self {
        Self {map: HashMap::new()}
    }

    pub fn deserialize(buf: &[u8]) -> Self {
        Self {map: deserialize(buf)}
    }

    pub fn serialize(&self) -> Vec<u8> {
        serialize(&self.map)
    }

    pub fn add(&mut self, key: PathBuf, hash: TubHash) {
        self.map.insert(key, TreeEntry::new(Kind::File, hash));
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::random_hash;

    #[test]
    fn test_kind() {
        for k in 0..4 {
            let kind: Kind = k.into();
            assert_eq!(kind as u8, k);
        }
        assert_eq!(Kind::Dir as u8, 0);
        assert_eq!(Kind::Dir, 0.into());
        assert_eq!(Kind::File as u8, 1);
        assert_eq!(Kind::File, 1.into());
        assert_eq!(Kind::ExFile as u8, 2);
        assert_eq!(Kind::ExFile, 2.into());
        assert_eq!(Kind::Symlink as u8, 3);
        assert_eq!(Kind::Symlink, 3.into());
    }

    #[test]
    #[should_panic (expected = "Unknown Kind: 5")]
    fn test_kind_panic() {
        let kind: Kind = 5.into();
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut map: TreeMap = HashMap::new();
        let pb = PathBuf::from("foo");
        let hash = [11_u8; TUB_HASH_LEN];
        map.insert(pb, TreeEntry::new_file(hash));
        let buf = serialize(&map);
        assert_eq!(buf, [11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
                         11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
                         11, 11, 1, 3, 102, 111, 111]
        );
        let map2 = deserialize(&buf);
        assert_eq!(map2, map);

        let mut map: TreeMap = HashMap::new();
        map.insert(PathBuf::from("as"), TreeEntry::new_file(random_hash()));
        map.insert(PathBuf::from("the"), TreeEntry::new_file(random_hash()));
        map.insert(PathBuf::from("world"), TreeEntry::new_file(random_hash()));
        let buf = serialize(&map);
        let map2 = deserialize(&buf);
        assert_eq!(map2, map);
    }
}
