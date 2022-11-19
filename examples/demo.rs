use std::fs::File;
use std::path::PathBuf;
use std::io;
use std::env;
use std::process::exit;
use bathtub_db::leaf_io::*;


fn main() -> io::Result<()> {
    let args = Vec::from_iter(env::args());
    if args.len() < 2 {
        eprintln!("Need path to file to hash");
        exit(42);
    }
    let path = PathBuf::from(&args[1]);
    let file = File::open(&path)?;
    let mut buf = new_leaf_buf();
    let mut lr = LeafReader::new(file);

    while let Some(info) = lr.read_next_leaf(&mut buf)? {
        println!("{} {} {}", info.as_db32(), buf.len(), info.index);
    }
    let root = lr.hash_root();
    println!("{} {}", root.as_db32(), root.size);
    Ok(())
}
