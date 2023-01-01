//! Doodles on version control software built on Bathtub DB

use std::collections::{HashMap, HashSet};
use std::path::{PathBuf, Path};
use std::fs;
use std::io;
use std::convert::Into;
use std::os::unix::fs::PermissionsExt;
use std::os::unix;
use std::io::prelude::*;

use crate::dbase32::db32enc;
use crate::leaf_io::TubBuf;
use crate::store::Store;
use crate::base::*;


const MAX_DEPTH: usize = 32;


#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Kind {
    EmptyDir,
    Dir,
    EmptyFile,
    File,
    ExeFile,
    SymLink,
}

impl From<u8> for Kind {
    fn from(item: u8) -> Self {
        match item {
            0 => Self::EmptyDir,
            1 => Self::Dir,
            2 => Self::EmptyFile,
            3 => Self::File,
            4 => Self::ExeFile,
            5 => Self::SymLink,
            _ => panic!("Unknown Kind: {}", item),
        }
    }
}


/// List of paths to be tracked
#[derive(Debug, PartialEq)]
pub struct TrackingList {
    set: HashSet<String>,
}

impl TrackingList {
    pub fn new () -> Self {
        Self {set: HashSet::new()}
    }

    pub fn deserialize(buf: &[u8]) -> Self {
        let mut tl = Self::new();
        let mut offset = 0;
        while offset < buf.len() {
            let size = u16::from_le_bytes(
                buf[offset..offset + 2].try_into().expect("oops")
            ) as usize;
            offset += 2;
            let path = String::from_utf8(
                buf[offset..offset + size].to_vec()
            ).unwrap();
            offset += size;
            tl.add(path);
        }
        assert_eq!(offset, buf.len());
        tl
    }

    pub fn serialize(&self, buf: &mut Vec<u8>) {
        for pb in self.as_sorted_vec() {
            let path = pb.as_bytes();
            let size = path.len() as u16;
            buf.extend_from_slice(&size.to_le_bytes());
            buf.extend_from_slice(path);
        }
    }

    pub fn as_sorted_vec(&self) -> Vec<&String> {
        let mut list = Vec::from_iter(self.set.iter());
        list.sort();
        list
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn contains(&self, pb: &String) -> bool {
        self.set.contains(pb)
    }

    pub fn add(&mut self, path: String) -> bool {
        self.set.insert(path)
    }

    pub fn remove(&mut self, path: &String) -> bool {
        self.set.remove(path)
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
}

pub type TreeMap = HashMap<String, TreeEntry>;


/// Stores entries in a directory
#[derive(Debug, PartialEq)]
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

    pub fn as_map(&self) -> &TreeMap {
        &self.map
    }

    pub fn deserialize(buf: &[u8]) -> Self {
        let mut map: TreeMap = HashMap::new();
        let mut offset = 0;
        while offset < buf.len() {
            let kind: Kind = buf[offset].into();
            let size = buf[offset + 1] as usize;
            assert!(size > 0);
            offset += 2;

            let name = String::from_utf8(buf[offset..offset+size].to_vec()).unwrap();
            offset += size;

            let h: TubHash = buf[offset..offset + TUB_HASH_LEN].try_into().expect("oops");
            offset += h.len();

            map.insert(name, TreeEntry::new(kind, h));
        }
        assert_eq!(offset, buf.len());
        Self {map: map}
    }

    pub fn serialize(&self, buf: &mut Vec<u8>) {
        let mut items = Vec::from_iter(self.map.iter());
        items.sort_by(|a, b| b.0.cmp(a.0));
        for (name, entry) in items.iter() {
            let path = name.as_bytes();
            let size = path.len() as u8;
            assert!(size > 0);
            buf.push(entry.kind as u8);
            buf.push(size);
            buf.extend_from_slice(path);
            buf.extend_from_slice(&entry.hash);
        }
    }

    fn add(&mut self, name: String, kind: Kind, hash: TubHash) {
        self.map.insert(name, TreeEntry::new(kind, hash));
    }

    pub fn add_empty_dir(&mut self, name: String) {
        self.add(name, Kind::EmptyDir, [0_u8; TUB_HASH_LEN]);
    }

    pub fn add_empty_file(&mut self, name: String) {
        self.add(name, Kind::EmptyFile, [0_u8; TUB_HASH_LEN]);
    }

    pub fn add_dir(&mut self, name: String, hash: TubHash) {
        self.add(name, Kind::Dir, hash);
    }

    pub fn add_file(&mut self, name: String, hash: TubHash) {
        self.add(name, Kind::File, hash);
    }

    pub fn add_exefile(&mut self, name: String, hash: TubHash) {
        self.add(name, Kind::ExeFile, hash);
    }

    pub fn add_symlink(&mut self, name: String, hash: TubHash) {
        self.add(name, Kind::SymLink, hash);
    }
}


pub struct Commit {
    tree: TubHash,
    msg: String,
}

impl Commit {
    pub fn new(tree: TubHash, msg: String) -> Self {
        Self {tree: tree, msg: msg}
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.tree);
        buf.extend_from_slice(&self.msg.as_bytes());
        buf
    }

    pub fn deserialize(buf: &Vec<u8>) -> Self {
        let tree: TubHash = buf[0..TUB_HASH_LEN].try_into().expect("oops");
        let msg = String::from_utf8(buf[TUB_HASH_LEN..].to_vec()).unwrap();
        Self {tree: tree, msg: msg}
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


pub struct Scanner {
    tbuf: TubBuf,
    obuf: Vec<u8>,
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            tbuf: TubBuf::new(),
            obuf: Vec::new(),
        }
    }

    fn scan_tree_inner(&mut self, dir: &Path, depth: usize) -> io::Result<Option<TubHash>>
    {
        if depth >= MAX_DEPTH {
            panic!("Depth {} is >= MAX_DEPTH {}", depth, MAX_DEPTH);
        }
        let mut tree = Tree::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let ft = entry.file_type()?;
            let path = entry.path();
            let name = path.file_name().unwrap().to_str().unwrap().to_string();
            if ft.is_symlink() {
                let value = fs::read_link(&path)?;
                let data = value.to_str().unwrap().as_bytes();
                let hash = self.tbuf.hash_data(ObjectType::Data, &data);
                tree.add_symlink(name, hash);
            }
            else if ft.is_file() {
                let meta = fs::metadata(&path)?;
                let size = meta.len();
                if size > 0 {
                    let file = fs::File::open(&path)?;
                    let hash = self.tbuf.hash_file(file, size)?;
                    if meta.permissions().mode() & 0o111 != 0 {  // Executable?
                        tree.add_exefile(name, hash);
                    }
                    else {
                        tree.add_file(name, hash);
                    }
                }
                else {
                    tree.add_empty_file(name);
                }
            }
            else if ft.is_dir() {
                if let Some(hash) = self.scan_tree_inner(&path, depth + 1)? {
                    tree.add_dir(name, hash);
                }
                else {
                    //println!("empty dir: {:?}", path);
                    tree.add_empty_dir(name);
                }
            }
        }
        if tree.len() > 0 {
            self.obuf.clear();
            tree.serialize(&mut self.obuf);
            let hash = self.tbuf.hash_data(ObjectType::Tree, &self.obuf);
            //eprintln!("{} {:?}", db32enc(&hash), dir);
            Ok(Some(hash))
        }
        else {
            Ok(None)
        }
    }

    pub fn scan_tree(&mut self, dir: &Path) -> io::Result<Option<TubHash>> {
        self.scan_tree_inner(dir, 0)
    }
}


fn commit_tree_inner(tub: &mut Store, dir: &Path, depth: usize)-> io::Result<Option<TubHash>>
{
    if depth >= MAX_DEPTH {
        panic!("Depth {} is >= MAX_DEPTH {}", depth, MAX_DEPTH);
    }
    let mut tree = Tree::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let path = entry.path();
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        if ft.is_symlink() {
            let value = fs::read_link(&path)?;
            let data = value.to_str().unwrap().as_bytes();
            let (hash, _new) = tub.add_object(data)?;
            tree.add_symlink(name, hash);
        }
        else if ft.is_file() {
            let meta = fs::metadata(&path)?;
            let size = meta.len();
            if size > 0 {
                let file = fs::File::open(&path)?;
                let (hash, _new) = tub.import_file(file, size)?;
                if meta.permissions().mode() & 0o111 != 0 {  // Executable?
                    tree.add_exefile(name, hash);
                }
                else {
                    tree.add_file(name, hash);
                }
            }
            else {
                tree.add_empty_file(name);
            }
        }
        else if ft.is_dir() {
            if let Some(hash) = commit_tree_inner(tub, &path, depth + 1)? {
                tree.add_dir(name, hash);
            }
            else {
                tree.add_empty_dir(name);
            }
        }
    }
    if tree.len() > 0 {
        let mut obj = Vec::new();
        tree.serialize(&mut obj);
        let (hash, _new) = tub.add_tree(&obj)?;
        //eprintln!("Tree: {} {:?}", db32enc(&hash), dir);
        Ok(Some(hash))
    }
    else {
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
    if depth >= MAX_DEPTH {
        panic!("Depth {} is >= MAX_DEPTH {}", depth, MAX_DEPTH);
    }
    if let Some(data) = store.get_object(root, false)? {
        let tree = Tree::deserialize(&data);
        fs::create_dir_all(&path)?;
        for (name, entry) in tree.as_map() {
            let mut pb = path.to_path_buf();
            pb.push(name);
            match entry.kind {
                Kind::EmptyDir => {
                    fs::create_dir_all(&pb)?;
                },
                Kind::Dir => {
                    eprintln!("D {:?}", pb);
                    restore_tree_inner(store, &entry.hash, &pb, depth + 1)?;
                },
                Kind::EmptyFile => {
                    fs::File::create(&pb)?;
                },
                Kind::File | Kind::ExeFile => {
                    if let Some(mut object) = store.open(&entry.hash)? {
                        let mut file = fs::File::create(&pb)?;
                        if entry.kind == Kind::ExeFile {
                            file.set_permissions(fs::Permissions::from_mode(0o755))?;
                            eprintln!("X {:?}", pb);
                        }
                        else {
                            eprintln!("F {:?}", pb);
                        }
                        object.write_to_file(&mut file)?;
                    } else {
                        panic!("could not find object {}", db32enc(&entry.hash));
                    }
                }
                Kind::SymLink => {
                    if let Some(buf) = store.get_object(&entry.hash, false)? {
                        eprintln!("S {:?}", &pb);
                        if let Ok(_) = fs::remove_file(&pb) {
                            // FIXME: handle this more better
                            eprintln!("Deleted old {:?}", &pb);
                        }
                        let s = String::from_utf8(buf).unwrap();
                        let target = PathBuf::from(s);
                        unix::fs::symlink(&target, &pb)?;
                    } else {
                        panic!("could not find symlink object {}", db32enc(&entry.hash));
                    }
                },
            }
        }
    } else {
        panic!("Could not find tree object {}", db32enc(root));
    }
    Ok(())
}

pub fn restore_tree(store: &mut Store, root: &TubHash, path: &Path) -> io::Result<()> {
    restore_tree_inner(store, root, path, 0)
}


#[derive(Debug)]
pub struct WorkingTree {
    store: Store,
}

impl WorkingTree {
    pub fn new(store: Store) -> Self {
        Self {store: store}
    }

    fn tl_path(&self) -> PathBuf {
        let mut pb = self.store.path();
        pb.push("tracking_list");
        pb
    }

    pub fn load_tracking_list(&self) -> io::Result<TrackingList> {
        let pb = self.tl_path();
        if let Ok(mut file) = fs::File::open(&pb) {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            Ok(TrackingList::deserialize(&buf))
        }
        else {
            Ok(TrackingList::new())
        }
    }

    pub fn save_tracking_list(&self, tl: TrackingList) -> io::Result<()> {
        let pb = self.tl_path();
        let mut file = fs::File::create(&pb)?;
        let mut buf = Vec::new();
        tl.serialize(&mut buf);
        file.write_all(&buf)?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use crate::util::random_hash;

    #[test]
    fn test_working_tree() {
        let (tmp, mut store) = Store::new_tmp();
        let wt = WorkingTree::new(store);
        let mut tl = wt.load_tracking_list().unwrap();
        assert_eq!(tl.len(), 0);
        wt.save_tracking_list(tl).unwrap();
        let tl = wt.load_tracking_list().unwrap();
        assert_eq!(tl.len(), 0);
    }

    #[test]
    fn test_kind() {
        for k in 0..4 {
            let kind: Kind = k.into();
            assert_eq!(kind as u8, k);
        }
        assert_eq!(Kind::EmptyDir as u8, 0);
        assert_eq!(Kind::EmptyDir, 0.into());
        assert_eq!(Kind::Dir as u8, 1);
        assert_eq!(Kind::Dir, 1.into());
        assert_eq!(Kind::EmptyFile as u8, 2);
        assert_eq!(Kind::EmptyFile, 2.into());
        assert_eq!(Kind::File as u8, 3);
        assert_eq!(Kind::File, 3.into());
        assert_eq!(Kind::ExeFile as u8, 4);
        assert_eq!(Kind::ExeFile, 4.into());
        assert_eq!(Kind::SymLink as u8, 5);
        assert_eq!(Kind::SymLink, 5.into());
    }

    #[test]
    #[should_panic(expected = "Unknown Kind: 6")]
    fn test_kind_panic1() {
        let _kind: Kind = 6.into();
    }

    #[test]
    #[should_panic(expected = "Unknown Kind: 255")]
    fn test_kind_panic2() {
        let _kind: Kind = 255.into();
    }

    #[test]
    fn test_tracking_list() {
        let mut tl  = TrackingList::new();
        assert_eq!(tl.len(), 0);
        let mut buf = Vec::new();
        tl.serialize(&mut buf);
        assert_eq!(buf, vec![]);
        assert_eq!(TrackingList::deserialize(&buf), tl);

        let pb = String::from("test");
        assert!(! tl.contains(&pb));
        tl.add(pb.clone());
        assert!(tl.contains(&pb));
        assert_eq!(tl.len(), 1);
        assert_eq!(tl.as_sorted_vec(), vec![&String::from("test")]);
        tl.serialize(&mut buf);
        assert_eq!(buf, vec![4, 0, 116, 101, 115, 116]);
        assert_eq!(TrackingList::deserialize(&buf), tl);

        let pb = String::from("foo");
        assert!(! tl.contains(&pb));
        tl.add(pb.clone());
        assert!(tl.contains(&pb));
        assert_eq!(tl.len(), 2);
        assert_eq!(tl.as_sorted_vec(), vec![
            &String::from("foo"),
            &String::from("test"),
        ]);
        buf.clear();
        tl.serialize(&mut buf);
        assert_eq!(buf, vec![
            3, 0, 102, 111, 111,
            4, 0, 116, 101, 115, 116,
        ]);
        assert_eq!(TrackingList::deserialize(&buf), tl);

        let pb = String::from("sparse");
        assert!(! tl.contains(&pb));
        tl.add(pb.clone());
        assert!(tl.contains(&pb));
        assert_eq!(tl.len(), 3);
        assert_eq!(tl.as_sorted_vec(), vec![
            &String::from("foo"),
            &String::from("sparse"),
            &String::from("test"),
        ]);
        buf.clear();
        tl.serialize(&mut buf);
        assert_eq!(buf, vec![
            3, 0, 102, 111, 111,
            6, 0, 115, 112, 97, 114, 115, 101,
            4, 0, 116, 101, 115, 116,
        ]);
        assert_eq!(TrackingList::deserialize(&buf), tl);
    }

    #[test]
    #[should_panic(expected = "Depth 32 is >= MAX_DEPTH 32")]
    fn test_commit_tree_depth_panic() {
        let (_tmp, mut store) = Store::new_tmp();
        let pb = PathBuf::from("word");
        commit_tree_inner(&mut store, &pb, MAX_DEPTH);
    }

    #[test]
    #[should_panic(expected = "Depth 32 is >= MAX_DEPTH 32")]
    fn test_restore_tree_depth_panic() {
        let (_tmp, mut store) = Store::new_tmp();
        let root = random_hash();
        let pb = PathBuf::from("word");
        restore_tree_inner(&mut store, &root, &pb, MAX_DEPTH);
    }

    #[test]
    fn test_serialize_deserialize() {
        /*
        let mut map: TreeMap = HashMap::new();

        let pb = PathBuf::from("bar");
        let hash = [11_u8; TUB_HASH_LEN];
        map.insert(pb, TreeEntry::new(Kind::File, hash));
        let buf = serialize(&map);
        assert_eq!(buf, [3,  3, 98, 97, 114,
                        11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
                        11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
                        11, 11]
        );
        let map2 = deserialize(&buf);
        assert_eq!(map2, map);

        let mut map: TreeMap = HashMap::new();
        map.insert(PathBuf::from("as"), TreeEntry::new(Kind::File, random_hash()));
        map.insert(PathBuf::from("the"), TreeEntry::new(Kind::File, random_hash()));
        map.insert(PathBuf::from("world"), TreeEntry::new(Kind::File, random_hash()));
        let buf = serialize(&map);
        let map2 = deserialize(&buf);
        assert_eq!(map2, map);
        */
    }
}
