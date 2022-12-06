//! Doodles on version control software built on Bathtub DB

use std::collections::HashMap;
use std::path::{PathBuf, Path};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::fs;
use std::io;


use crate::dbase32::db32enc_str;
use crate::base::*;


pub type TreeMap = HashMap<PathBuf, TubHash>;


const MAX_DEPTH: usize = 32;


pub enum EntryType {
    Dir,
    File,
    ExFile,
    SymLink,
}


pub struct SrcFile {
    pub path: PathBuf,
    pub size: u64,
}

impl SrcFile {
    pub fn new(path: PathBuf, size: u64) -> Self {
        Self {
            path: path,
            size: size,
        }
    }

    pub fn open(&self) -> io::Result<fs::File> {
        fs::File::open(&self.path)
    }
}



pub fn build_tree_state(dir: &Path) -> io::Result<()> {
    println!("Yo");
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let path = entry.path();
        println!("{:?}", path.file_name().unwrap());
    }
    Ok(())
}


fn scan_files(dir: &Path, accum: &mut Vec<SrcFile>, depth: usize) -> io::Result<u64> {
    let mut total: u64 = 0;
    if depth < MAX_DEPTH {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let ft = entry.file_type()?;
            let path = entry.path();
            if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
                eprintln!("Skipping hiddin: {:?}", path);
            }
            else if ft.is_symlink() {
                eprintln!("Skipping symlink: {:?}", path);
            }
            else if ft.is_file() {
                let size = fs::metadata(&path)?.len();
                if size > 0 {
                    accum.push(SrcFile::new(path, size));
                    total += size;
                }
                else {
                    eprintln!("Skipping empty file: {:?}", path);
                }
            }
            else if ft.is_dir() {
                total += scan_files(&path, accum, depth + 1)?;
            }
        }
    }
    Ok(total)
}


pub struct Scanner {
    accum: Vec<SrcFile>,
    total: u64,
}

impl Scanner {
    pub fn scan_dir(dir: &Path) -> io::Result<Scanner> {
        let mut accum: Vec<SrcFile> = Vec::new();
        let total = scan_files(dir, &mut accum, 0)?;
        Ok(Scanner {accum: accum, total: total})
    }

    pub fn iter(&self) -> std::slice::Iter<SrcFile> {
        self.accum.iter()
    }

    pub fn count(&self) -> usize {
        self.accum.len()
    }

    pub fn total(&self) -> u64 {
        self.total
    }
}



pub fn deserialize(buf: &[u8]) -> TreeMap {
    let mut map: TreeMap = HashMap::new();
    let mut offset = 0;
    while offset < buf.len() {
        let sbuf = buf[offset..offset + 2].try_into().unwrap();
        let size = u16::from_le_bytes(sbuf) as usize;
        offset += 2;
        let s = OsStr::from_bytes(&buf[offset..offset+size]);
        let pb = PathBuf::from(s);
        offset += size;
        let h: TubHash = buf[offset..offset + TUB_HASH_LEN].try_into().expect("oops");
        offset += TUB_HASH_LEN;
        map.insert(pb, h);
    }
    assert_eq!(offset, buf.len());
    map
}


pub fn serialize(map: &TreeMap) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut items = Vec::from_iter(map.iter());
    items.sort_by(|a, b| b.0.cmp(a.0));
    for (p, h) in items.iter() {
        println!("{:?} {}", p, db32enc_str(*h));
        let path = p.to_str().unwrap().as_bytes();
        let size = path.len() as u16;
        buf.extend_from_slice(&size.to_le_bytes());
        buf.extend_from_slice(path);
        buf.extend_from_slice(&h[..]);
    }
    buf
}


pub struct Tree {
    map: HashMap<PathBuf, TubHash>,
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
        self.map.insert(key, hash);
    }
}
