//! Test fixtures.

// (FIXME: should eventually be put somewhere else).

use std::io::prelude::*;
use std::path::PathBuf;
use std::fs;
use tempfile;




pub struct TestTempDir {
    tmp: tempfile::TempDir,
}


impl TestTempDir {
    pub fn new() -> Self {
        Self {
            tmp: tempfile::TempDir::new().unwrap(),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.tmp.path().to_path_buf()
    }

    // Construct an absolute path starting with self.path()
    pub fn build(&self, names: &[&str]) -> PathBuf {
        let mut p = self.path();
        for n in names {
            p.push(n);
        }
        p
    }

    pub fn list_dir(&self, parts: &[&str]) -> Vec<String> {
        let path = self.build(parts);
        let mut names: Vec<String> = Vec::new();
        if let Ok(entries) = fs::read_dir(path) {
            for result in entries {
                if let Ok(entry) = result {
                    names.push(
                        entry.file_name().into_string().unwrap()
                    );
                    println!("{:?}", entry.file_name());
                }
            }
        }
        names.sort();
        names
    }

    pub fn list_root(&self) -> Vec<String> {
        self.list_dir(&[])
    }

    pub fn read(&self, names: &[&str]) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        fs::File::open(self.build(names)).unwrap().read_to_end(&mut buf).unwrap();
        buf
    }

    pub fn write(&self, names: &[&str], data: &[u8]) {
        fs::File::create(self.build(names)).unwrap().write_all(data).unwrap();
    }

    pub fn touch(&self, names: &[&str]) {
        fs::File::create(self.build(names)).unwrap();
    }

    pub fn mkdir(&self, names: &[&str]) {
        fs::create_dir(self.build(names)).unwrap();
    }

    pub fn makedirs(&self, names: &[&str]) {
        fs::create_dir_all(self.build(names)).unwrap();
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tempdir() {
        let tmp = TestTempDir::new();
        let base = tmp.path();
        let a = tmp.build(&["a"]);
        assert!(a.to_str().unwrap().starts_with(base.to_str().unwrap()));
        assert!(a.to_str().unwrap().ends_with("/a"));

        assert_eq!(tmp.list_root().len(), 0);
        tmp.touch(&["a"]);
        assert_eq!(tmp.list_root(), vec!["a"]);
    
    }
}

