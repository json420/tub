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

    let mut file = File::open(&path)?;
    let root = store.import_file(file)?;
    println!("{}", root.as_db32());
    store.finalize_tmp(tmp, &root.hash)?;
    Ok(())
}
