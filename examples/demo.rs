use std::io;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::prelude::*;
use tub::util::random_hash;
use tub::dbase32::db32enc;
use tub::blockchain::{BlockChain};
use tub::base::*;
use tub::store::Store;
use tub::dvcs::{Scanner, commit_tree, WorkingTree};
use zstd::bulk;


fn main() -> io::Result<()> {
    let (_tmp, mut store) = Store::new_tmp();
    /*
    let mut scanner = Scanner::new();
    let p = Path::new("/usr/share/doc");
    let hash1 = scanner.scan_tree(&p)?.unwrap();
    let hash2 = commit_tree(&mut store, &p)?;
    println!("yo");
    assert_eq!(hash1, hash2);
    */

    let wt = WorkingTree::new(store);
    let mut tl = wt.load_tracking_list()?;
    tl.add(PathBuf::from("hello"));
    tl.add(PathBuf::from("apples"));
    wt.save_tracking_list(tl)?;

    let tl = wt.load_tracking_list()?;

    for p in tl.as_sorted_vec() {
        println!("{:?}", p);
    }
    println!("more yo {}", tl.len());
    Ok(())
}
