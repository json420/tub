use std::io;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use tub::util::random_hash;
use tub::dbase32::db32enc;
use tub::blockchain::{BlockChain};
use tub::base::*;
use tub::store::Store;
use tub::dvcs::{Scanner, commit_tree};
use zstd::bulk;


fn main() -> io::Result<()> {
    let mut scanner = Scanner::new();
    let p = Path::new("/usr/share/doc");
    let hash1 = scanner.scan_tree(&p)?.unwrap();
    let (_tmp, mut store) = Store::new_tmp();
    let hash2 = commit_tree(&mut store, &p)?;
    println!("yo");
    assert_eq!(hash1, hash2);
    Ok(())
}
