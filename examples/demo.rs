use std::io;
use tub::chaos::{DefaultName, DefaultObject, DefaultStore};
use tub::inception::Fanout;
use tub::helpers::TestTempDir;
use tub::util::getrandom;
use tub::blockchain::{KeyBlock, Block};


const COUNT: usize = 65536;

fn main() -> io::Result<()> {
    let tmp = TestTempDir::new();
    let file = tmp.create(&["some_file.store"]);
    let mut store = DefaultStore::new(file);
    let mut obj = DefaultObject::new();
    obj.reset(96, 0);
    let mut kb = KeyBlock::new(obj.as_mut_data());
    kb.generate();
    let id = obj.finalize();
    println!("{}", id);
    store.save(&mut obj)?;
    //let b = kb.into_block();
    //let pk = kb.pubkey();
    //let mut kb = KeyBlock::new(obj.as_mut_data());
    Ok(())
}
