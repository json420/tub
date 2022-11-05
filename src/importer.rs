use std::path::{Path, PathBuf};
use std::fs::{read_dir, metadata, File};


const MAX_DEPTH: usize = 32;

pub struct SrcFile(pub PathBuf, pub u64);

fn scan_files<P: AsRef<Path>>(dir: P, accum: &mut Vec<SrcFile>, depth: usize) -> u64 {
    if depth < MAX_DEPTH {
        let mut total: u64 = 0;
        if let Ok(entries) = read_dir(dir) {
            for entry in entries {
                let e = entry.unwrap();
                let ft = e.file_type().unwrap();
                let path = e.path();
                if ft.is_file() {
                    let size = metadata(&path).unwrap().len();
                    accum.push(SrcFile(path, size));
                    total += size;
                }
                else if ft.is_dir() {
                    total += scan_files(path, accum, depth + 1);
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
    pub fn scan_dir<P: AsRef<Path>>(dir: P) -> Self {
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
    use tempfile::TempDir;
    use crate::helpers;
    use std::fs::{create_dir_all, File};
    use std::io::prelude::*;

    fn mk_tmp_path(tmp: &TempDir, names: &[&str]) -> PathBuf {
        let mut path = tmp.path().to_path_buf();
        for n in names {
            path.push(n);
        }
        path
    }

    fn mk_dir(tmp: &TempDir, names: &[&str]) {
        create_dir_all(mk_tmp_path(tmp, names)).unwrap();
    }

    fn mk_file(tmp: &TempDir, names: &[&str]) -> File {
        File::create(mk_tmp_path(tmp, names)).unwrap()
    }

    fn create_test_dirs(tmp: &TempDir) {
        mk_dir(tmp, &["A", "B", "C"]);
    }

    fn create_test_files(tmp: &TempDir) {
        File::create(
            tmp.path().join("Z")
        ).unwrap().write_all(&[0_u8; 7]);
        File::create(
            tmp.path().join("A").join("Y")
        ).unwrap().write_all(&[1_u8; 11]);
    }


    #[test]
    fn test_scan_files() {
        let tmp = TempDir::new().unwrap();

        let mut accum: Vec<SrcFile> = Vec::new();
        scan_files(tmp.path(), &mut accum, MAX_DEPTH + 1);
        assert_eq!(accum.len(), 0);

        // Contains directories but no files
        create_test_dirs(&tmp);
        assert_eq!(scan_files(tmp.path(), &mut accum, 0), 0);
        assert_eq!(accum.len(), 0);

        // Contains files but called at MAX_DEPTH
        create_test_files(&tmp);
        assert_eq!(scan_files(tmp.path(), &mut accum, MAX_DEPTH), 0);
        assert_eq!(accum.len(), 0);

        // All good, should find test files
        assert_eq!(scan_files(tmp.path(), &mut accum, 0), 18);
        assert_eq!(accum.len(), 2);
    }

    #[test]
    fn test_scanresult() {
        //let tmp = TempDir::new().unwrap();
        let tmp = helpers::TestTempDir::new();

        // Empty directory
        let s = Scanner::scan_dir(tmp.path());
        assert_eq!(s.count(), 0);
        assert_eq!(s.total(), 0);

        // Contains directories but no files
        //create_test_dirs(&tmp);
        tmp.makedirs(&["A", "B", "C"]);
        let s = Scanner::scan_dir(tmp.path());
        assert_eq!(s.count(), 0);
        assert_eq!(s.total(), 0);

        // Contains files
        //create_test_files(&tmp);
        tmp.write(&["Z"], &[0_u8; 7]);
        tmp.write(&["A", "B", "C", "Y"], &[1_u8; 11]);
        let s = Scanner::scan_dir(tmp.path());
        assert_eq!(s.count(), 2);
        assert_eq!(s.total(), 18);
    }
}
