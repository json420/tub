//! A far Superior *pository.

use std::path::{Path, PathBuf};
use std::{io, fs};
use std::io::prelude::*;
use crate::base::*;
use crate::dbase32::DirNameIter;


macro_rules! other_err {
    ($msg:literal) => {
        Err(io::Error::new(io::ErrorKind::Other, $msg))
    }
}


/// Insert a suppository layout into an empty DOTDIR directory.
pub fn init_suppository(path: &Path) -> io::Result<Suppository>
{
    let mut pb = PathBuf::from(path);

    // objects directory and sub-directories
    pb.push(OBJECTDIR);
    fs::create_dir(pb.as_path())?;
    for name in DirNameIter::new() {
        pb.push(name);
        fs::create_dir(pb.as_path())?;
        pb.pop();
    }
    pb.pop();

    // partial directory:
    pb.push(PARTIALDIR);
    fs::create_dir(pb.as_path())?;
    pb.pop();

    // tmp directory:
    pb.push(TMPDIR);
    fs::create_dir(pb.as_path())?;
    pb.pop();

    // REAMDE file  :-)
    pb.push(README);
    let mut f = fs::File::create(pb.as_path())?;
    f.write_all(README_CONTENTS)?;
    pb.pop();

    Suppository::new(pb)
}



pub fn find_suppository(path: &Path) -> io::Result<Suppository>
{
    let mut pb = PathBuf::from(path);
    loop {
        pb.push(DOTDIR);
        if pb.is_dir() {
            return Ok(Suppository::new(pb)?);
        }
        pb.pop();
        if !pb.pop() {
            return other_err!("cannot find control directory");
        }
    }
}


/// Suppository: short for "Superior Repository".
pub struct Suppository {
    dir: PathBuf,
}

impl Suppository {
    pub fn new(dir: PathBuf) -> io::Result<Self> {
        Ok(Self {dir: dir})
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::TestTempDir;

    #[test]
    fn test_find_suppository() {
        let tmp = TestTempDir::new();

        // We're gonna use these over and over:
        let tree = tmp.pathbuf();
        let dotdir = tmp.build(&[DOTDIR]);
        let foo = tmp.build(&["foo"]);
        let bar = tmp.build(&["foo", "bar"]);
        let child = tmp.build(&[DOTDIR, "a", "child"]);
        let empty: Vec<String> = vec![];

        // tmp.path() is an empty directory still:
        assert!(find_suppository(&tree).is_err());
        assert!(find_suppository(&dotdir).is_err());
        assert!(find_suppository(&foo).is_err());
        assert!(find_suppository(&bar).is_err());
        assert!(find_suppository(&child).is_err());

        // Nothing should have been created
        assert_eq!(tmp.list_root(), empty);

        // create foo/bar, but still no DOTDIR
        assert_eq!(tmp.makedirs(&["foo", "bar"]), bar);

        assert!(find_suppository(&tree).is_err());
        assert!(find_suppository(&dotdir).is_err());
        assert!(find_suppository(&foo).is_err());
        assert!(find_suppository(&bar).is_err());
        assert!(find_suppository(&child).is_err());

        // Still nothing should have been created by find_suppository():
        assert_eq!(tmp.list_root(), ["foo"]);
        assert_eq!(tmp.list_dir(&["foo"]), ["bar"]);
        assert_eq!(tmp.list_dir(&["foo", "bar"]), empty);

        // create DOTDIR
        assert_eq!(tmp.makedirs(&[DOTDIR]), dotdir);
        assert!(find_suppository(&tree).is_ok());
        assert!(find_suppository(&dotdir).is_ok());
        assert!(find_suppository(&foo).is_ok());
        assert!(find_suppository(&bar).is_ok());
    }

    #[test]
    fn test_init_supository() {
        let tmp = TestTempDir::new();
        let mut pb = PathBuf::from(tmp.pathbuf());
        init_suppository(&mut pb).unwrap();
        let mut expected = vec![OBJECTDIR, PARTIALDIR, TMPDIR, README];//, PACKFILE];
        expected.sort();
        assert_eq!(tmp.list_root(), expected);
        let dirs = tmp.list_dir(&[OBJECTDIR]);
        assert_eq!(dirs.len(), 1024);
        let expected: Vec<String> = DirNameIter::new().collect();
        assert_eq!(dirs, expected);
        assert_eq!(dirs[0], "33");
        assert_eq!(dirs[1], "34");
        assert_eq!(dirs[1022], "YX");
        assert_eq!(dirs[1023], "YY");
    }
}
