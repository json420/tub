//! Doodles on version control software built on Bathtub DB

use std::collections::HashMap;
use std::path::{PathBuf, Path};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::fs;
use std::io;
use std::convert::Into;
use std::io::prelude::*;
use std::os::unix::fs::PermissionsExt;
use std::os::unix;

use crate::dbase32::db32enc_str;
use crate::leaf_io::TubBuf;
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

    pub fn new_dir(hash: TubHash) -> Self {
        Self {kind: Kind::Dir, hash: hash}
    }

    pub fn new_file(hash: TubHash) -> Self {
        Self {kind: Kind::File, hash: hash}
    }
}

pub type TreeMap = HashMap<PathBuf, TreeEntry>;


pub fn deserialize(buf: &[u8]) -> TreeMap {
    let mut map: TreeMap = HashMap::new();
    let mut offset = 0;
    while offset < buf.len() {
        let h: TubHash = buf[offset..offset + TUB_HASH_LEN].try_into().expect("oops");
        offset += h.len();

        let kind: Kind = buf[offset].into();
        let size = buf[offset + 1] as usize;
        assert!(size > 0);
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
        //println!("{:?} {}", p, db32enc_str(&entry.hash));
        let path = p.to_str().unwrap().as_bytes();
        let size = path.len() as u8;
        assert!(size > 0);
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

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn deserialize(buf: &[u8]) -> Self {
        Self {map: deserialize(buf)}
    }

    pub fn serialize(&self) -> Vec<u8> {
        serialize(&self.map)
    }

    pub fn add_dir(&mut self, key: PathBuf, hash: TubHash) {
        self.map.insert(key, TreeEntry::new_dir(hash));
    }

    pub fn add_file(&mut self, key: PathBuf, hash: TubHash) {
        self.map.insert(key, TreeEntry::new_file(hash));
    }

    pub fn add(&mut self, key: PathBuf, kind: Kind, hash: TubHash) {
        self.map.insert(key, TreeEntry::new(kind, hash));
    }
}


pub struct TreeFile {
    pub path: PathBuf,
    pub size: u64,
    pub hash: TubHash,
}

impl TreeFile {
    pub fn new(path: PathBuf, size: u64, hash: TubHash) -> Self {
        Self {path: path, size: size, hash: hash}
    }

    pub fn is_large(&self) -> bool {
        self.size > LEAF_SIZE
    }

    pub fn open(&self) -> io::Result<fs::File> {
        fs::File::open(&self.path)
    }
}

pub struct TreeDir {
    pub data: Vec<u8>,
    pub hash: TubHash,
}

impl TreeDir {
    pub fn new(data: Vec<u8>, hash: TubHash) -> Self {
        Self {data: data, hash: hash}
    }
}


pub struct TreeAccum {
    pub trees: Vec<TreeDir>,
    pub files: Vec<TreeFile>,
}

impl TreeAccum {
    pub fn new() -> Self {
        Self {
            trees: Vec::new(),
            files: Vec::new(),
        }
    }
}


fn scan_tree_inner(accum: &mut TreeAccum, dir: &Path, depth: usize)-> io::Result<Option<TubHash>>
{
    if depth < MAX_DEPTH {
        let mut tree = Tree::new();
        let mut tbuf = TubBuf::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let ft = entry.file_type()?;
            let path = entry.path();
            let name = path.file_name().unwrap();
            if name.to_str().unwrap().starts_with(".") {
                eprintln!("Skipping hiddin: {:?}", path);
            }
            else if ft.is_symlink() {
                eprintln!("Skipping symlink: {:?}", path);
            }
            else if ft.is_file() {
                let meta = fs::metadata(&path)?;
                let size = meta.len();
                if size > 0 {
                    let mut file = fs::File::open(&path)?;
                    let hash = tbuf.hash_file(file, size)?;
                    tree.add_file(PathBuf::from(name), hash);
                    accum.files.push(
                        TreeFile::new(path.to_path_buf(), size, hash)
                    );
                }
            }
            else if ft.is_dir() {
                if let Some(hash) = scan_tree_inner(accum, &path, depth + 1)? {
                    tree.add_dir(PathBuf::from(name), hash);
                }
            }
        }
        if tree.len() > 0 {
            let obj = tree.serialize();
            let hash = tbuf.hash_data(ObjectType::Tree, &obj);
            accum.trees.push(TreeDir::new(obj, hash));
            eprintln!("{} {:?}", db32enc_str(&hash), dir);
            Ok(Some(hash))
        }
        else {
            Ok(None)
        }
    }
    else {
        panic!("max depth reached");
        Ok(None)
    }
}

pub fn scan_tree(dir: &Path) -> io::Result<(TubHash, TreeAccum)> {
    let mut accum = TreeAccum::new();
    if let Some(root_hash) = scan_tree_inner(&mut accum, dir, 0)? {
        Ok((root_hash, accum))
    }
    else {
        panic!("FIXME: handle this more better");
    }
}


fn commit_tree_inner(tub: &mut Store, dir: &Path, depth: usize)-> io::Result<Option<TubHash>>
{
    if depth < MAX_DEPTH {
        let mut tree = Tree::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let ft = entry.file_type()?;
            let path = entry.path();
            let name = path.file_name().unwrap();
            if name.to_str().unwrap().starts_with(".") {
                eprintln!("Skipping hiddin: {:?}", path);
            }
            else if ft.is_symlink() {
                eprintln!("S: {:?}", path);
                let value = fs::read_link(&path)?;
                let data = value.to_str().unwrap().as_bytes();
                let (hash, _new) = tub.add_object(data)?;
                tree.add(PathBuf::from(name), Kind::Symlink, hash);
            }
            else if ft.is_file() {
                let meta = fs::metadata(&path)?;
                let size = meta.len();
                if size > 0 {
                    let mut file = fs::File::open(&path)?;
                    let (hash, _new) = tub.import_file(file, size)?;
                    let is_executable = meta.permissions().mode() & 0o111 != 0;
                    let kind = if is_executable {Kind::ExFile} else {Kind::File};
                    tree.add(PathBuf::from(name), kind, hash);
                }
            }
            else if ft.is_dir() {
                if let Some(hash) = commit_tree_inner(tub, &path, depth + 1)? {
                    tree.add_dir(PathBuf::from(name), hash);
                }
            }
        }
        if tree.len() > 0 {
            let obj = tree.serialize();
            let (hash, _new) = tub.add_tree(&obj)?;
            eprintln!("Tree: {} {:?}", db32enc_str(&hash), dir);
            Ok(Some(hash))
        }
        else {
            Ok(None)
        }
    }
    else {
        panic!("max depth reached");
        Ok(None)
    }
}

pub fn commit_tree(tub: &mut Store, dir: &Path) -> io::Result<TubHash> {
    if let Some(root_hash) = commit_tree_inner(tub, dir, 0)? {
        Ok(root_hash)
    }
    else {
        panic!("FIXME: handle this more better");
    }
}


fn restore_tree_inner(store: &mut Store, root: &TubHash, path: &Path, depth: usize) -> io::Result<()> {
    if depth < MAX_DEPTH {
        if let Some(data) = store.get_object(root, false)? {
            let map = deserialize(&data);
            fs::create_dir_all(&path)?;
            for (name, entry) in map.iter() {
                let mut pb = path.to_path_buf();
                pb.push(name);
                match entry.kind {
                    Kind::Dir => {
                        //println!("D {:?}", pb);
                        restore_tree_inner(store, &entry.hash, &pb, depth + 1)?;
                    },
                    Kind::File => {
                        if let Some(mut object) = store.open(&entry.hash)? {
                            println!("F {:?}", pb);
                            let mut file = fs::File::create(&pb)?;
                            object.write_to_file(&mut file)?;
                        } else {
                            panic!("could not find object {}", db32enc_str(&entry.hash));
                        }
                    }
                    Kind::ExFile => {
                        if let Some(mut object) = store.open(&entry.hash)? {
                            println!("X {:?}", pb);
                            let mut file = fs::File::create(&pb)?;
                            file.set_permissions(fs::Permissions::from_mode(0o755))?;
                            object.write_to_file(&mut file)?;
                        } else {
                            panic!("could not find object {}", db32enc_str(&entry.hash));
                        }
                    }
                    Kind::Symlink => {
                        if let Some(buf) = store.get_object(&entry.hash, false)? {
                            println!("S {:?}", pb);
                            let s = String::from_utf8(buf).unwrap();
                            let target = PathBuf::from(s);
                            unix::fs::symlink(&target, &pb)?;
                        } else {
                            panic!("could not find symlink object {}", db32enc_str(&entry.hash));
                        }
                    },
                }
            }
        } else {
            panic!("could not find tree {}", db32enc_str(root));
        }
    }
    Ok(())
}

pub fn restore_tree(store: &mut Store, root: &TubHash, path: &Path) -> io::Result<()> {
    restore_tree_inner(store, root, path, 0)
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
    #[should_panic(expected = "Unknown Kind: 4")]
    fn test_kind_panic1() {
        let _kind: Kind = 4.into();
    }

    #[test]
    #[should_panic(expected = "Unknown Kind: 255")]
    fn test_kind_panic2() {
        let _kind: Kind = 255.into();
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
