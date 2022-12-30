use std::io;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use tub::util::random_hash;
use tub::dbase32::db32enc;
use tub::blockchain::{BlockChain};
use tub::base::*;
use tub::dvcs::Scanner;
use zstd::bulk;


fn main() -> io::Result<()> {
    let mut scanner = Scanner::new();
    let p = Path::new("/usr/share/doc");
    let hash = scanner.scan_tree(&p);
    println!("yo");
    Ok(())
}
