use std::io;
use std::time::Instant;
use tub::chaos::DefaultObject;
use tub::blockchain::Block;
use tub::util::getrandom;
//use sodium
use sodiumoxide;


const COUNT: usize = 100_000;

fn main() -> io::Result<()> {
    sodiumoxide::init().unwrap();
    let (pk, sk) = sodiumoxide::crypto::sign::gen_keypair();

    let mut obj = DefaultObject::new();
    obj.reset(124, 0);
    getrandom(obj.as_mut_data());
    let mut block: Block<30> = Block::new(obj.as_mut_data(), pk);

    println!("🛁 Signing {} times...", COUNT);
    let start = Instant::now();
    for _ in 0..COUNT {
        block.sign(&sk);
    }
    let elapsed = start.elapsed().as_secs_f64();
    let rate = COUNT as f64 / elapsed;
    println!("🚀 {} blocks signed per second", rate as u64);

    println!("");

    println!("🛁 Veriying {} times...", COUNT);
    let start = Instant::now();
    for _ in 0..COUNT {
        block.verify();
    }
    let elapsed = start.elapsed().as_secs_f64();
    let rate = COUNT as f64 / elapsed;
    println!("🚀 {} blocks verified per second", rate as u64);

    Ok(())
}

/*

real	0m18.719s
user	0m16.292s


*/
