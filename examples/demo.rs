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
    Ok(())
}
