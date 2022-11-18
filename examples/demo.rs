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
    let mut i = 0;
    loop {
        if !lr.read_next_leaf(&mut buf)? {
            break;
        }
        println!("{} {}", i, buf.len());
        i += 1;
    }
    println!("done");
    Ok(())
}
