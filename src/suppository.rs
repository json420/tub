//! A far Superior *pository.

use std::path::{Path, PathBuf};
use std::io;
use crate::base::DOTDIR;


macro_rules! other_err {
    ($msg:literal) => {
        Err(io::Error::new(io::ErrorKind::Other, $msg))
    }
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
}
