use std::path::PathBuf;
use std::fs;
use std::io;
use bathtub_db::util::random_hash;
use bathtub_db::blockchain::{Block, Chain};


fn main() -> io::Result<()> {
    let pb = PathBuf::from("demo.bc");
    let file = fs::File::options()
                        .read(true)
                        .append(true)
                        .create(true).open(&pb)?;
    let mut chain = Chain::new(file);
    let mut block = Block::new();

    for i in 0..100 {
        let h = random_hash();
        block.set_payload_hash(&h);
    }

    println!("demotastic");
    //chain.read_next_block(&mut block)?;
    chain.verify_chain()?;
    Ok(())
}
