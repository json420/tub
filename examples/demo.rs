use std::path::PathBuf;
use std::fs;
use std::io;
use bathtub_db::util::random_hash;
use bathtub_db::dbase32::db32enc_str;
use bathtub_db::blockchain::{BlockChain};

use std::time::SystemTime;


fn main() -> io::Result<()> {
    let mut bc = BlockChain::generate();
    for i in 0..100 {
        bc.append(&random_hash());
        println!("{} {}", db32enc_str(bc.as_payload()), bc.counter());
    }
    Ok(())
}
