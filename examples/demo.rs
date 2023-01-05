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
    for _ in 0..1_000_000 {
        obj.randomize(true);
        assert!(obj.info().size() <= 64 * 1024);
        //println!("{} {} {}", obj.hash(), obj.info().kind(), obj.info().size());
        store.save(&obj)?;
    }
    */
    println!("{}", store.len());
    let keys = store.keys();
    for name in keys {
        //println!("{}", name);
        store.load(&name, &mut obj)?;
        //println!("{} *", obj.hash());
    }
    Ok(())
}
