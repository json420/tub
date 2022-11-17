//! Import from the file system.

use std::path::{Path, PathBuf};
use std::fs::{read_dir, metadata, File};
use std::io;


const MAX_DEPTH: usize = 32;

pub struct SrcFile(pub PathBuf, pub u64);

impl SrcFile {
    pub fn open(&self) -> io::Result<File> {
        File::open(&self.0)
    }
}


fn scan_files(dir: &Path, accum: &mut Vec<SrcFile>, depth: usize) -> u64 {
    if depth < MAX_DEPTH {
        let mut total: u64 = 0;
        if let Ok(entries) = read_dir(dir) {
            for entry in entries {
                let e = entry.unwrap();
                let ft = e.file_type().unwrap();
                let path = e.path();
                if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
                    eprintln!("Skipping hiddin: {:?}", path);
                }
                else if ft.is_symlink() {
                    eprintln!("Skipping symlink: {:?}", path);
                }
                else if ft.is_file() {
                    let size = metadata(&path).unwrap().len();
                    if size > 0 {
                        accum.push(SrcFile(path, size));
                        total += size;
                    }
                    else {
                        eprintln!("Skipping empty file: {:?}", path);
                    }
                }
                else if ft.is_dir() {
                    total += scan_files(&path, accum, depth + 1);
                }
            }
        }
        total
    }
    else {
        0
    }
}


pub struct Scanner {
    accum: Vec<SrcFile>,
    total: u64,
}

impl Scanner {
    pub fn scan_dir(dir: &Path) -> Self {
        let mut accum: Vec<SrcFile> = Vec::new();
        let total = scan_files(dir, &mut accum, 0);
        Scanner {accum: accum, total: total}
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::TestTempDir;
    use std::fs::{create_dir_all, File};
    use std::io::prelude::*;

    fn mk_test_dirs(tmp: &TestTempDir) {
        tmp.makedirs(&["A", "B", "C"]);
    }

    fn mk_test_files(tmp: &TestTempDir) {
        tmp.write(&["Z"], &[0_u8; 7]);
        tmp.write(&["A", "B", "C", "Y"], &[1_u8; 11]);
    }

    #[test]
    fn test_scan_files() {
        let tmp = TestTempDir::new();

        let mut accum: Vec<SrcFile> = Vec::new();
        scan_files(tmp.path(), &mut accum, MAX_DEPTH + 1);
        assert_eq!(accum.len(), 0);

        // Contains directories but no files
        mk_test_dirs(&tmp);
        assert_eq!(scan_files(tmp.path(), &mut accum, 0), 0);
        assert_eq!(accum.len(), 0);

        // Contains files but called at MAX_DEPTH
        mk_test_files(&tmp);
        assert_eq!(scan_files(tmp.path(), &mut accum, MAX_DEPTH), 0);
        assert_eq!(accum.len(), 0);

        // All good, should find test files
        assert_eq!(scan_files(tmp.path(), &mut accum, 0), 18);
        assert_eq!(accum.len(), 2);
    }

    #[test]
    fn test_scanresult() {
        let tmp = TestTempDir::new();

        // Empty directory
        let s = Scanner::scan_dir(tmp.path());
        assert_eq!(s.count(), 0);
        assert_eq!(s.total(), 0);

        // Contains directories but no files
        mk_test_dirs(&tmp);
        
        let s = Scanner::scan_dir(tmp.path());
        assert_eq!(s.count(), 0);
        assert_eq!(s.total(), 0);

        // Contains files
        mk_test_files(&tmp);
        let s = Scanner::scan_dir(tmp.path());
        assert_eq!(s.count(), 2);
        assert_eq!(s.total(), 18);
    }
}
