use tub::chaos::*;
use tub::protocol::Blake3;
use std::io;
use std::path;


fn main() -> io::Result<()> {
    let p = path::Path::new("newnew.tub");
    let file = open_for_store(p)?;
    let mut store: Store<Blake3, 30> = Store::new(file);
    let mut obj = store.new_object();
    store.reindex(&mut obj)?;
    println!("{}", store.len());
    /*
    println!("{}", obj.hash());
    for _ in 0..69_000 {
        obj.randomize(true);
        assert!(obj.info().size() <= 64 * 1024);
        println!("{} {} {}", obj.hash(), obj.info().size(), obj.is_valid());
        store.save(&obj);
    }
    */
    println!("{}", store.len());
    Ok(())
}
