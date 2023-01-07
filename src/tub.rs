//! Higher level repository built on `chaos`.

use std::path::{Path, PathBuf};
use std::{io, fs};
use std::io::prelude::*;
use crate::base::*;
use crate::dbase32::DirNameIter;
use crate::protocol::Hasher;
use crate::protocol::DefaultHasher;
use crate::chaos::Store;



pub type DefaultTub = Tub<DefaultHasher, 30>;

pub fn create_dotdir(path: &Path) -> io::Result<PathBuf>
{
    let mut pb = PathBuf::from(path);
    pb.push(DOTDIR);
    fs::create_dir(&pb)?;
    Ok(pb)
}

pub fn find_dotdir(path: &Path) -> Option<PathBuf> {
    let mut pb = PathBuf::from(path);
    loop {
        pb.push(DOTDIR);
        if pb.is_dir() {
            return Some(pb);
        }
        pb.pop();
        if !pb.pop() {
            return None;
        }
    }
}

pub fn create_store(path: &Path) -> io::Result<fs::File> {
    fs::File::options().read(true).append(true).create_new(true).open(path)
}

pub fn open_store(path: &Path) -> io::Result<fs::File> {
    fs::File::options().read(true).append(true).open(path)
}


/// Tub: controls control directory; a repository.
pub struct Tub<H: Hasher, const N: usize> {
    dotdir: PathBuf,
    filename: PathBuf,
    store: Store<H, N>,
}

impl<H: Hasher, const N: usize> Tub<H, N> {
    pub fn create(parent: &Path) -> io::Result<Self> {
        let dotdir = create_dotdir(parent)?;
        let mut filename = dotdir.clone();
        filename.push(PACKFILE);
        let file = create_store(&filename)?;
        let store = Store::<H, N>::new(file);
        Ok( Self {dotdir: dotdir, filename: filename, store: store} )
    }

    pub fn open(dotdir: PathBuf) -> io::Result<Self> {
        let mut filename = dotdir.clone();
        filename.push(PACKFILE);
        let file = open_store(&filename)?;
        let store = Store::<H, N>::new(file);
        Ok( Self {dotdir: dotdir, filename: filename, store: store} )
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::TestTempDir;

    #[test]
    fn test_create_dotdir() {
        let tmp = TestTempDir::new();
        let r = create_dotdir(tmp.path());
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), tmp.build(&[DOTDIR]));
        assert_eq!(tmp.list_root(), &[DOTDIR]);
    }

    #[test]
    fn test_open_store() {
        let tmp = TestTempDir::new();
        let pb = tmp.build(&["a_store_file"]);
        let empty: Vec<String> = vec![];

        // Should fail if file is missing (and should not create anything)
        let r = open_store(&pb);
        assert!(r.is_err());
        assert_eq!(tmp.list_root(), empty);

        // Now try when file exists
        tmp.touch(&["a_store_file"]);
        let r = open_store(&pb);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().metadata().unwrap().len(), 0);
    }

    #[test]
    fn test_create_store() {
        let tmp = TestTempDir::new();
        let pb = tmp.build(&["a_store_file"]);
        let empty: Vec<String> = vec![];

        let r = create_store(&pb);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().metadata().unwrap().len(), 0);
        assert_eq!(tmp.list_root(), &["a_store_file"]);

        // Should fail if file already exists
        let r = create_store(&pb);
        assert!(r.is_err());
        assert_eq!(tmp.list_root(), &["a_store_file"]);

        // Make sure we can open with open_store()
        let r = open_store(&pb);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().metadata().unwrap().len(), 0);
    }

    #[test]
    fn test_find_dotdir() {
        let tmp = TestTempDir::new();

        // We're gonna use these over and over:
        let tree = tmp.pathbuf();
        let dotdir = tmp.build(&[DOTDIR]);
        let foo = tmp.build(&["foo"]);
        let bar = tmp.build(&["foo", "bar"]);
        let child = tmp.build(&[DOTDIR, "a", "child"]);
        let empty: Vec<String> = vec![];

        // tmp.path() is an empty directory still:
        assert!(find_dotdir(&tree).is_none());
        assert!(find_dotdir(&dotdir).is_none());
        assert!(find_dotdir(&foo).is_none());
        assert!(find_dotdir(&bar).is_none());
        assert!(find_dotdir(&child).is_none());

        // Nothing should have been created
        assert_eq!(tmp.list_root(), empty);

        // create foo/bar, but still no DOTDIR
        assert_eq!(tmp.makedirs(&["foo", "bar"]), bar);

        assert!(find_dotdir(&tree).is_none());
        assert!(find_dotdir(&dotdir).is_none());
        assert!(find_dotdir(&foo).is_none());
        assert!(find_dotdir(&bar).is_none());
        assert!(find_dotdir(&child).is_none());

        // Still nothing should have been created by find_dotdir():
        assert_eq!(tmp.list_root(), ["foo"]);
        assert_eq!(tmp.list_dir(&["foo"]), ["bar"]);
        assert_eq!(tmp.list_dir(&["foo", "bar"]), empty);

        // create DOTDIR
        assert_eq!(tmp.makedirs(&[DOTDIR]), dotdir);
        assert!(find_dotdir(&tree).is_some());
        assert!(find_dotdir(&dotdir).is_some());
        assert!(find_dotdir(&foo).is_some());
        assert!(find_dotdir(&bar).is_some());
    }

    #[test]
    fn test_tub_create() {
        let tmp = TestTempDir::new();
        assert!(DefaultTub::create(tmp.path()).is_ok());

        // Should fail if it already exists:
        let r = DefaultTub::create(tmp.path());
        assert!(r.is_err());

        // Make sure we can open what we created
        assert!(DefaultTub::open(tmp.build(&[DOTDIR])).is_ok());
    }

    #[test]
    fn test_tub_open() {
        let tmp = TestTempDir::new();
        let dotdir = tmp.build(&[DOTDIR]);

        // Should fail if DOTDIR doesn't exist
        assert!(DefaultTub::open(dotdir.clone()).is_err());

        // Should likewise fail if PACKFILE doesnt' exist
        tmp.mkdir(&[DOTDIR]);
        assert!(DefaultTub::open(dotdir.clone()).is_err());

        // Now it should work
        tmp.touch(&[DOTDIR, PACKFILE]);
        assert!(DefaultTub::open(dotdir.clone()).is_ok());
    }
}

