use std::{fs, io};
use std::time::Instant;
use tub::helpers::TestTempDir;
use tub::chaos::Name;
use tub::blockchain::Chain;
use sodiumoxide;


const COUNT: usize = 1_000_000;

fn main() -> io::Result<()> {
    sodiumoxide::init();

    //let tmp = TestTempDir::new();
    //let file = tmp.create(&["block.chain"]);
    let file = fs::File::options().read(true).append(true).open("block.chain")?;
    let (sk, mut chain) = Chain::generate(file);
    chain.verify()?;

    let mut payload: Name<30> = Name::new();
    for _ in 0..100_000 {
        payload.randomize();
        let hash = chain.sign_next(&payload, &sk)?;
        println!("{}", hash);
    }
    Ok(())
}
