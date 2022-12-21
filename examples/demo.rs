use std::io;
use std::fs::File;
use std::io::prelude::*;
use tub::util::random_hash;
use tub::dbase32::db32enc;
use tub::blockchain::{BlockChain};
use tub::base::*;
use zstd::bulk;


fn main() -> io::Result<()> {
    /*
    let mut bc = BlockChain::generate();
    for _i in 0..100 {
        bc.append(BlockType::Configure, &random_hash());
        println!("{} {}", db32enc(bc.as_payload()), bc.counter());
    }
    */
    let mut infile = File::open(".tub/append.tub")?;
    let mut outfile = File::create(".tub/append.tub.zstd")?;
    let mut buf: Vec<u8> = Vec::new();
    buf.resize(LEAF_SIZE as usize, 0);
    let mut i = 0;
    let mut insize = 0;
    let mut outsize = 0;
    loop {
        let size = infile.read(&mut buf)?;
        if size == 0 {
            break;
        }
        insize += size;
        println!("{} {}", i, size);
        i += 1;
        buf.resize(size, 0);
        let comp = bulk::compress(&buf, 0)?;
        outsize += comp.len();
        outfile.write_all(&comp)?;
    }
    println!("{} --> {}", insize, outsize);
    Ok(())
}
