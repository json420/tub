use std::io;
use bathtub_db::util::random_hash;
use bathtub_db::dbase32::db32enc;
use bathtub_db::blockchain::{BlockChain};
use bathtub_db::base::*;


fn main() -> io::Result<()> {
    let mut bc = BlockChain::generate();
    for _i in 0..100 {
        bc.append(BlockType::Configure, &random_hash());
        println!("{} {}", db32enc(bc.as_payload()), bc.counter());
    }
    Ok(())
}
