use std::io;
use std::time::Instant;
use tub::chaos::DefaultObject;
use tub::blockchain::Block;
use tub::util::getrandom;
use rand::rngs::OsRng;
use ed25519_dalek::{SigningKey, Signature, Signer, VerifyingKey, Verifier};


const COUNT: usize = 100_000;

fn main() -> io::Result<()> {
    
    let mut csprng = OsRng;
    let sk = SigningKey::generate(&mut csprng);
    let pk = sk.verifying_key();

    let mut obj = DefaultObject::new();
    obj.reset(124, 0);
    getrandom(obj.as_mut_data());
    let mut block: Block = Block::new(pk);

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
