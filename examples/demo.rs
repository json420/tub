use std::io;
use tub::util::random_hash;
use tub::dbase32::db32enc;
use tub::blockchain::{BlockChain};
use tub::base::*;


fn main() -> io::Result<()> {
    let mut bc = BlockChain::generate();
    for _i in 0..100 {
        bc.append(BlockType::Configure, &random_hash());
        println!("{} {}", db32enc(bc.as_payload()), bc.counter());
    }
    Ok(())
}
