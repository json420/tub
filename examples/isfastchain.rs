use std::{fs, io};
use std::time::Instant;
use tub::helpers::TestTempDir;
use tub::chaos::DefaultObject;
use tub::blockchain::{WriteBlock, ReadBlock};
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
    let mut writer: WriteBlock<30> = WriteBlock::new(obj.as_mut_data(), sk);

    println!("🛁 Signing {} times...", COUNT);
    let start = Instant::now();
    for _ in 0..COUNT {
        writer.sign();
    }
    let elapsed = start.elapsed().as_secs_f64();
    let rate = COUNT as f64 / elapsed;
    println!("🚀 {} blocks signed per second", rate as u64);

    println!("");

    let obj = obj;
    println!("🛁 Veriying {} times...", COUNT);
    let reader: ReadBlock<30> = ReadBlock::new(obj.as_data(), pk);
    let start = Instant::now();
    for _ in 0..COUNT {
        reader.is_valid();
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
