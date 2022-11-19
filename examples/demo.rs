use std::fs::File;
use std::path::PathBuf;
use std::io;
use std::env;
use std::process::exit;
use bathtub_db::leaf_io::*;
use bathtub_db::store::{Store, init_store};


fn main() -> io::Result<()> {
    let args = Vec::from_iter(env::args());
    if args.len() < 2 {
        eprintln!("Need path to file to hash");
        exit(42);
    }
    let path = PathBuf::from(&args[1]);

    let (tmpdir, mut store) = Store::new_tmp();
    let tmp = store.allocate_tmp()?;
    println!("{:?}", tmp.path);
    let mut reader = LeafReader::new(File::open(&path)?);
    let mut buf = new_leaf_buf();
    while let Some(info) = reader.read_next_leaf(&mut buf)? {
        println!("{}", info.index);
    }
    let root = reader.hash_root();
    println!("{}", root.as_db32());
    store.finalize_tmp(tmp, &root.hash)?;
    Ok(())
}
