use std::io;
use tub::chaos::{DefaultName, DefaultObject, DefaultStore};
use tub::inception::Fanout;
use tub::helpers::TestTempDir;
use tub::util::getrandom;
use tub::blockchain::Chain;


const COUNT: usize = 65536;

fn main() -> io::Result<()> {
    let tmp = TestTempDir::new();
    let file = tmp.create(&["some_file.store"]);

    let (sk, mut chain) = Chain::generate(file)?;
    chain.verify()?;
    let mut name = DefaultName::new();
    println!("{}", chain.header.hash());
    for _ in 0..COUNT {
        name.randomize();
        chain.sign_next(&name, &sk)?;
        println!("{} {}", chain.block.hash(), chain.block.previous());
    }
    //chain.verify();

    let file = chain.into_file();
    let mut chain = Chain::open(file)?;
    //chain.verify();
    Ok(())
}
