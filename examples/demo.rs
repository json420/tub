use std::io;
use tub::chaos::DefaultName;
use tub::helpers::TestTempDir;
use tub::blockchain::Chain;


const COUNT: usize = 65536;

fn main() -> io::Result<()> {
    let tmp = TestTempDir::new();
    let file = tmp.create(&["some_file.store"]);

    let mut chain = Chain::generate(file)?;
    chain.verify()?;
    let mut name = DefaultName::new();
    println!("{}", chain.header.hash());
    for _ in 0..COUNT {
        name.randomize();
        chain.sign_next(&name)?;
        println!("{} {}", chain.block.hash(), chain.block.previous());
    }
    //chain.verify();

    let file = chain.into_file();
    let mut chain = Chain::open(file)?;
    for i in 0..65536 {
       assert!(chain.load_block_at(i)?);
    }
    //chain.verify();
    chain.seek_to_beyond();
    while chain.load_previous()? {
        println!("{} {}", chain.block.hash(), chain.block.index());
    }
    Ok(())
}
