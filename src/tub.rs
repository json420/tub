//! Higher level repository built on `chaos`.

use std::path::{Path, PathBuf};
use std::io::prelude::*;
use std::io::Result as IoResult;
use std::fs::{File, create_dir};
use crate::base::*;
use crate::protocol::{Hasher, DefaultHasher};
use crate::chaos::{Object, Store, Name};
use crate::blockchain::Chain;
use crate::dvcs::TrackingList;

pub type DefaultTub = Tub<DefaultHasher, 30>;


pub fn create_dotdir(path: &Path) -> IoResult<PathBuf>
{
    let mut pb = PathBuf::from(path);
    pb.push(DOTDIR);
    create_dir(&pb)?;
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

pub fn create_for_append(path: &Path) -> IoResult<File> {
    File::options().read(true).append(true).create_new(true).open(path)
}

pub fn open_for_append(path: &Path) -> IoResult<File> {
    File::options().read(true).append(true).open(path)
}

pub struct HashingFileReaderIter {
    size: u64,
    remaining: u64,
    file: File,
}


/// Put all your 🏴‍☠️ treasure in here, matey! 💰💵🦓
pub struct Tub<H: Hasher, const N: usize> {
    dotdir: PathBuf,
    treedir: PathBuf,
    pub store: Store<H, N>,
}

impl<H: Hasher, const N: usize> Tub<H, N> {
    pub fn dotdir(&self) -> &Path {
        &self.dotdir
    }

    pub fn treedir(&self) -> &Path {
        &self.treedir
    }

    pub fn create(parent: &Path) -> IoResult<Self> {
        let dotdir = create_dotdir(parent)?;
        let mut filename = dotdir.clone();
        filename.push(PACKFILE);
        let file = create_for_append(&filename)?;
        let store = Store::<H, N>::new(file);
        Ok( Self {dotdir, treedir: parent.to_owned(), store} )
    }

    pub fn open(dotdir: PathBuf) -> IoResult<Self> {
        let mut filename = dotdir.clone();
        filename.push(PACKFILE);
        let file = open_for_append(&filename)?;
        let store = Store::<H, N>::new(file);
        let mut treedir = dotdir.clone();
        treedir.pop();
        Ok( Self {dotdir, treedir, store} )
    }

    pub fn idx_file(&self) -> IoResult<File> {
        let mut pb = self.dotdir.clone();
        pb.push(INDEX_FILE);
        if let Ok(file) = open_for_append(&pb) {
            Ok(file)
        }
        else {
            create_for_append(&pb)
        }
    }

    pub fn join(&self, dir: &str, hash: &Name<N>) -> PathBuf {
        let mut pb = self.dotdir.clone();
        pb.push(dir);
        pb.push(hash.to_string());
        pb
    }

    pub fn check(&mut self) -> IoResult<()> {
        let mut obj: Object<H, N> = Object::new();
        self.store.reindex(&mut obj)
    }

    pub fn reindex(&mut self) -> IoResult<()> {
        let mut obj: Object<H, N> = Object::new();
        self.store.reindex_from(&mut obj, self.idx_file()?)?;
        Ok(())
    }

    pub fn create_branch(&self) -> IoResult<Chain> {
        let mut filename = self.dotdir.clone();
        filename.push("fixme.branch");
        let file = create_for_append(&filename)?;
        let chain = Chain::generate(file)?;
        // Save secret key:
        filename.pop();
        filename.push("omg.fixme.soon");
        let file = create_for_append(&filename)?;
        chain.save_secret_key(file)?;
        Ok(chain)
    }

    pub fn open_branch(&self) -> IoResult<Chain> {
        let mut filename = self.dotdir.clone();
        filename.push("fixme.branch");
        let file = open_for_append(&filename)?;
        Chain::open(file)
    }

    pub fn load_branch_seckey(&self, chain: &mut Chain) -> IoResult<bool> {
        let mut filename = self.dotdir.clone();
        filename.push("omg.fixme.soon");
        if let Ok(file) = File::open(&filename) {
            chain.load_secret_key(file)
        }
        else {
            Ok(false)
        }
    }

    pub fn load_tracking_list(&self, obj: &mut Object<H, N>) -> IoResult<TrackingList> {
        let mut filename = self.dotdir.clone();
        filename.push("staged.tub");
        obj.clear();
        if let Ok(mut file) = File::open(&filename) {
            if file.read_exact(obj.as_mut_header()).is_ok() {
                obj.resize_to_info();
                file.read_exact(obj.as_mut_data())?;
                if ! obj.is_valid() {
                    panic!("Invalid object: {}", obj.hash());
                }
            }
        }
        Ok(
            TrackingList::deserialize(obj.as_data())
        )
    }

    pub fn save_tracking_list(&self, obj: &mut Object<H, N>, tl: &TrackingList) -> IoResult<()> {
        let mut filename = self.dotdir.clone();
        filename.push("staged.tub");
        let mut file = File::create(&filename)?;
        obj.clear();
        tl.serialize(obj.as_mut_vec());
        obj.finalize_with_kind(0);
        file.write_all(obj.as_buf())?;
        file.flush()
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
    fn test_open_for_append() {
        let tmp = TestTempDir::new();
        let pb = tmp.build(&["a_store_file"]);
        let empty: Vec<String> = vec![];

        // Should fail if file is missing (and should not create anything)
        let r = open_for_append(&pb);
        assert!(r.is_err());
        assert_eq!(tmp.list_root(), empty);

        // Now try when file exists
        tmp.touch(&["a_store_file"]);
        let r = open_for_append(&pb);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().metadata().unwrap().len(), 0);
    }

    #[test]
    fn test_create_for_append() {
        let tmp = TestTempDir::new();
        let pb = tmp.build(&["a_store_file"]);

        let r = create_for_append(&pb);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().metadata().unwrap().len(), 0);
        assert_eq!(tmp.list_root(), &["a_store_file"]);

        // Should fail if file already exists
        let r = create_for_append(&pb);
        assert!(r.is_err());
        assert_eq!(tmp.list_root(), &["a_store_file"]);

        // Make sure we can open with open_for_append()
        let r = open_for_append(&pb);
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

