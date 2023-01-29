//! Doodles on version control software built on Bathtub DB

use std::collections::{HashMap, HashSet};
use std::path::{PathBuf, Path};
use std::fs;
use std::io;
use std::convert::Into;
use std::os::unix::fs::PermissionsExt;
use std::os::unix;

use crate::protocol::{Hasher, Blake3};
use crate::chaos::{Object, Store, Name};
use crate::inception::{import_file, restore_file, hash_file};


const MAX_DEPTH: usize = 32;
pub type DefaultScanner = Scanner<Blake3, 30>;
pub type DefaultCommit = Commit<30>;


#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Kind {
    EmptyDir,
    EmptyFile,
    Dir,
    File,
    ExeFile,
    SymLink,
}

impl From<u8> for Kind {
    fn from(item: u8) -> Self {
        match item {
            0 => Self::EmptyDir,
            1 => Self::EmptyFile,
            2 => Self::Dir,
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


#[derive(Debug)]
pub struct Commit<const N: usize> {
    pub tree: Name<N>,
    pub msg: String,
}

impl<const N: usize> Commit<N> {
    pub fn new(tree: Name<N>, msg: String) -> Self {
        Self {tree: tree, msg: msg}
    }

    pub fn deserialize(buf: &[u8]) -> Self {
        Self {
            tree: Name::from(&buf[0..N]),
            msg: String::from_utf8(buf[N..].to_vec()).unwrap(),
        }
    }

    pub fn serialize(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self.tree.as_buf());
        buf.extend_from_slice(self.msg.as_bytes());
    }
}


#[derive(Debug, PartialEq)]
pub struct TreeEntry<const N: usize> {
    pub kind: Kind,
    pub hash: Name<N>,
}

impl<const N: usize> TreeEntry<N> {
    pub fn new(kind: Kind, hash: Name<N>) -> Self {
        Self {kind: kind, hash: hash}
    }
}


/// Stores entries in a directory
#[derive(Debug, PartialEq)]
pub struct Tree<const N: usize> {
    map: HashMap<String, TreeEntry<N>>,
}

impl<const N: usize> Tree<N> {
    pub fn new() -> Self {
        Self {map: HashMap::new()}
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn as_map(&self) -> &HashMap<String, TreeEntry<N>> {
        &self.map
    }

    pub fn deserialize(buf: &[u8]) -> Self {
        let mut map: HashMap<String, TreeEntry<N>> = HashMap::new();
        let mut offset = 0;
        while offset < buf.len() {
            let kind: Kind = buf[offset].into();
            let size = buf[offset + 1] as usize;
            assert!(size > 0);
            offset += 2;

            let name = String::from_utf8(buf[offset..offset+size].to_vec()).unwrap();
            offset += size;

            let h = Name::from(&buf[offset..offset + N]);
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
            buf.extend_from_slice(entry.hash.as_buf());
        }
    }

    fn add(&mut self, name: String, kind: Kind, hash: Name<N>) {
        self.map.insert(name, TreeEntry::new(kind, hash));
    }

    pub fn add_empty_dir(&mut self, name: String) {
        self.add(name, Kind::EmptyDir, Name::<N>::new());
    }

    pub fn add_empty_file(&mut self, name: String) {
        self.add(name, Kind::EmptyFile, Name::<N>::new());
    }

    pub fn add_dir(&mut self, name: String, hash: Name<N>) {
        self.add(name, Kind::Dir, hash);
    }

    pub fn add_file(&mut self, name: String, hash: Name<N>) {
        self.add(name, Kind::File, hash);
    }

    pub fn add_exefile(&mut self, name: String, hash: Name<N>) {
        self.add(name, Kind::ExeFile, hash);
    }

    pub fn add_symlink(&mut self, name: String, hash: Name<N>) {
        self.add(name, Kind::SymLink, hash);
    }
}


#[derive(Debug, PartialEq)]
pub enum ScanMode {
    Scan,
    Import,
}


pub struct Scanner<H: Hasher, const N: usize> {
    mode: ScanMode,
    obj: Object<H, N>,
    store: Store<H, N>,
    dir: PathBuf,
}

impl<H: Hasher, const N: usize> Scanner<H, N> {
    pub fn new(store: Store<H, N>, dir: &Path) -> Self {
        Self {
            mode: ScanMode::Scan,
            obj: Object::<H, N>::new(),
            store: store,
            dir: dir.to_path_buf(),
        }
    }

    pub fn into_store(self) -> Store<H, N> {
        self.store
    }

    pub fn enable_import(&mut self) {
        self.mode = ScanMode::Import;
    }

    fn scan_tree_inner(&mut self, dir: &Path, depth: usize) -> io::Result<Option<Name<N>>>
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
                let value = fs::read_link(&path)?;  // FIXME: Let's just store this value in the tree
                let data = value.to_str().unwrap().as_bytes();
                self.obj.clear();
                self.obj.as_mut_vec().extend_from_slice(&data);
                let hash = self.obj.finalize();
                println!("S {} {:?}", hash, path);
                tree.add_symlink(name, hash);
                if self.mode == ScanMode::Import {
                    self.store.save(&self.obj)?;
                }
            }
            else if ft.is_file() {
                let meta = fs::metadata(&path)?;
                let size = meta.len();
                if size > 0 {
                    let file = fs::File::open(&path)?;
                    let hash = match self.mode {
                        ScanMode::Scan => {
                            hash_file(&mut self.obj, file, size)?
                        }
                        ScanMode::Import => {
                            import_file(&mut self.store, &mut self.obj, file, size)?
                        }
                    };
                    if meta.permissions().mode() & 0o111 != 0 {  // Executable?
                        println!("X {} {:?}", hash, path);
                        tree.add_exefile(name, hash);
                    }
                    else {
                        println!("F {} {:?}", hash, path);
                        tree.add_file(name, hash);
                    }
                }
                else {
                    println!("EF {:?}", path);
                    tree.add_empty_file(name);
                }
            }
            else if ft.is_dir() {
                if let Some(hash) = self.scan_tree_inner(&path, depth + 1)? {
                    println!("D {} {:?}", hash, path);
                    tree.add_dir(name, hash);
                }
                else {
                    println!("ED {:?}", path);
                    tree.add_empty_dir(name);
                }
            }
        }
        if tree.len() > 0 {
            self.obj.clear();
            tree.serialize(self.obj.as_mut_vec());
            let hash = self.obj.finalize();
            if self.mode == ScanMode::Import {
                self.store.save(&self.obj)?;
            }
            Ok(Some(hash))
        }
        else {
            Ok(None)
        }
    }

    pub fn scan_tree(&mut self) -> io::Result<Option<Name<N>>> {
        let dir = self.dir.clone();
        self.scan_tree_inner(&dir, 0)
    }

    fn restore_tree_inner(&mut self, root: &Name<N>, path: &Path, depth: usize) -> io::Result<()> {
        if depth >= MAX_DEPTH {
            panic!("Depth {} is >= MAX_DEPTH {}", depth, MAX_DEPTH);
        }
        if self.store.load(root, &mut self.obj)? {
            let tree = Tree::deserialize(&self.obj.as_data());
            fs::create_dir_all(&path)?;
            for (name, entry) in tree.as_map() {
                let mut pb = path.to_path_buf();
                pb.push(name);
                match entry.kind {
                    Kind::EmptyDir => {
                        fs::create_dir_all(&pb)?;
                    },
                    Kind::EmptyFile => {
                        fs::File::create(&pb)?;
                    },
                    Kind::Dir => {
                        self.restore_tree_inner(&entry.hash, &pb, depth + 1)?;
                    },
                    Kind::File | Kind::ExeFile => {
                        if self.store.load(&entry.hash, &mut self.obj)? {
                            let mut file = fs::File::create(&pb)?;
                            if entry.kind == Kind::ExeFile {
                                file.set_permissions(fs::Permissions::from_mode(0o755))?;
                            }
                            restore_file(
                                &mut self.store, &mut self.obj, &mut file, &entry.hash
                            )?;
                        } else {
                            panic!("could not find object {}", entry.hash);
                        }
                    }
                    Kind::SymLink => {
                        if self.store.load(&entry.hash, &mut self.obj)? {
                            if let Ok(_) = fs::remove_file(&pb) {
                                eprintln!("Deleted old {:?}", &pb);  // FIXME: handle this more better
                            }
                            let s = String::from_utf8(self.obj.as_data().to_vec()).unwrap();
                            let target = PathBuf::from(s);
                            unix::fs::symlink(&target, &pb)?;
                        } else {
                            panic!("could not find symlink object: {}", &entry.hash);
                        }
                    },
                }
            }
        } else {
            panic!("Could not find tree object {}", root);
        }
        Ok(())
    }

    pub fn restore_tree(&mut self, root: &Name<N>) -> io::Result<()> {
        let dir = self.dir.clone();
        self.restore_tree_inner(root, &dir, 0)
    }
}


#[derive(Debug, PartialEq)]
pub enum Item<const N: usize> {
    EmptyDir,
    EmptyFile,
    Dir(Name<N>),
    File(Name<N>),
    ExeFile(Name<N>),
    SymLink(String),
}

#[inline]
fn item_to_kind<const N: usize>(item: &Item<N>) -> Kind {
    match item {
        Item::EmptyDir => {
            Kind::EmptyDir
        }
        Item::EmptyFile => {
            Kind::EmptyFile
        }
        Item::Dir(hash) => {
            Kind::Dir
        }
        Item::File(hash) => {
            Kind::File
        }
        Item::ExeFile(hash) => {
            Kind::ExeFile
        }
        Item::SymLink(target) => {
            Kind::SymLink
        }
    }
}

pub type ItemMap<const N: usize> = HashMap<String, Item<N>>;


/// Stores entries in a directory
#[derive(Debug, PartialEq)]
pub struct Tree2<const N: usize> {
    map: ItemMap<N>,
}

impl<const N: usize> Tree2<N> {
    pub fn new() -> Self {
        Self {map: HashMap::new()}
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn as_map(&self) -> &HashMap<String, Item<N>> {
        &self.map
    }

    pub fn deserialize(buf: &[u8]) -> Self {
        let mut map = HashMap::new();
        let mut offset = 0;
        while offset < buf.len() {
            let kind: Kind = buf[offset].into();
            let size = buf[offset + 1] as usize;
            assert!(size > 0);
            offset += 2;
            offset += 2;

            let key = String::from_utf8(
                buf[offset..offset + size].to_vec()
            ).unwrap();
            offset += size;

            let val: Item<N> = match kind {
                Kind::EmptyDir => {
                    Item::EmptyDir
                }
                Kind::EmptyFile => {
                    Item::EmptyFile
                }
                Kind::Dir | Kind::File | Kind::ExeFile => {
                    let hash = Name::from(&buf[offset..offset + N]);
                    offset += hash.len();
                    match kind {
                        Kind::Dir => {
                            Item::Dir(hash)
                        }
                        Kind::File => {
                            Item::File(hash)
                        }
                        Kind::ExeFile => {
                            Item::ExeFile(hash)
                        }
                        _ => {panic!("nope")}
                    }
                }
                Kind::SymLink => {
                    let size = u16::from_le_bytes(
                        buf[offset..offset + 2].try_into().unwrap()
                    ) as usize;
                    offset += 2;
                    let target = String::from_utf8(
                        buf[offset.. offset + size].to_vec()
                    );
                    offset += size;
                    Item::EmptyDir
                }
            };
            map.insert(key, val);
        }
        Self {map: map}
    }

    pub fn serialize(&self, buf: &mut Vec<u8>) {
        let mut pairs = Vec::from_iter(self.map.iter());
        pairs.sort_by(|a, b| b.0.cmp(a.0));
        for (name, item) in pairs.iter() {
            let kind = item_to_kind(&item);
            let name = name.as_bytes();
            let size = name.len() as u8; 
            buf.push(kind as u8);
            buf.push(size);
            buf.extend_from_slice(name);
            match item {
                Item::EmptyDir | Item::EmptyFile => {
                    // Nothing to do
                }
                Item::Dir(hash) | Item::File(hash) | Item::ExeFile(hash) => {
                    buf.extend_from_slice(hash.as_buf());
                }
                Item::SymLink(target) => {
                    let tsize = target.len() as u16;
                    buf.extend_from_slice(&tsize.to_le_bytes());
                    buf.extend_from_slice(target.as_bytes());
                }
            }
            
        }
    }
}



pub enum Status {
    Changed,
    Missing,
}


pub fn compare_tree<const N:usize>(a: ItemMap<N>, b: ItemMap<N>)
        -> Vec<(String, Status)>
{
    let mut ret: Vec<(String, Status)> = Vec::new();
    let mut keys = Vec::from_iter(a.keys());
    keys.sort();
    let keys = keys;
    for path in keys.iter() {
        let p = path.clone();  // FIXME
        let old = a.get(p).unwrap();
        if let Some(new) = b.get(p) {
            if new != old {
                ret.push((p.to_string(), Status::Changed));
            }
        }
        else {
            ret.push((p.to_string(), Status::Missing));
        }
    }
    ret
}

/*

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
*/


#[cfg(test)]
mod tests {
    use super::*;

/*
    #[test]
    fn test_working_tree() {
        let (_tmp, store) = Store::new_tmp();
        let wt = WorkingTree::new(store);
        let tl = wt.load_tracking_list().unwrap();
        assert_eq!(tl.len(), 0);
        wt.save_tracking_list(tl).unwrap();
        let tl = wt.load_tracking_list().unwrap();
        assert_eq!(tl.len(), 0);
    }
*/
    #[test]
    fn test_kind() {
        for k in 0..4 {
            let kind: Kind = k.into();
            assert_eq!(kind as u8, k);
        }
        assert_eq!(Kind::EmptyDir as u8, 0);
        assert_eq!(Kind::EmptyDir, 0.into());
        assert_eq!(Kind::EmptyFile as u8, 1);
        assert_eq!(Kind::EmptyFile, 1.into());
        assert_eq!(Kind::Dir as u8, 2);
        assert_eq!(Kind::Dir, 2.into());
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
}

