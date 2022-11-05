use std::path::{Path, PathBuf};
use std::fs::read_dir;


const MAX_DEPTH: usize = 32;

fn scan_files<P: AsRef<Path>>(dir: P, accum: &mut Vec<PathBuf>, depth: usize) {
    assert!(depth <= MAX_DEPTH + 1);
    if depth > MAX_DEPTH {
        return;
    }
    if let Ok(entries) = read_dir(dir) {
        for entry in entries {
            let e = entry.unwrap();
            let ft = e.file_type().unwrap();
            let path = e.path();
            if ft.is_file() {
                accum.push(path);
            }
            else if ft.is_dir() {
                scan_files(path, accum, depth + 1);
            }
        }
    }
}


pub struct ScanResult {
    accum: Vec<PathBuf>,
}

impl ScanResult {
    fn scan_dir<P: AsRef<Path>>(dir: P) -> Self {
        let mut accum: Vec<PathBuf> = Vec::new();
        scan_files(dir, &mut accum, 0);
        accum.sort();
        ScanResult {accum: accum}
    }

    fn iter(&self) -> std::slice::Iter<PathBuf> {
        self.accum.iter()
    }

    fn len(&self) -> usize {
        self.accum.len()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_files() {
        let mut accum: Vec<PathBuf> = Vec::new();
        scan_files("/no/such/dir", &mut accum, MAX_DEPTH + 1);
        assert_eq!(accum.len(), 0);
    }
}
