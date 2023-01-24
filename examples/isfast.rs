use std::io;
use std::time::Instant;
use tub::chaos::{DefaultObject, DefaultStore};
use tub::helpers::TestTempDir;


const COUNT: usize = 1_000_000;
const LOOPS: usize = 3;

fn main() -> io::Result<()> {
    let tmp = TestTempDir::new();
    let file = tmp.create(&["some_file.store"]);
    let mut store = DefaultStore::new(file);
    let mut obj = DefaultObject::new();

    println!("🤔 Is Tub 🛁 fast? 🚀");
    println!("");

    println!("🛁 Saving {} random 16-256 byte sized objects...", COUNT);
    let start = Instant::now();
    for _ in 0..COUNT {
        obj.randomize(true);
        store.save(&mut obj)?;
    }
    let elapsed = start.elapsed().as_secs_f64();
    let rate = COUNT as f64 / elapsed;
    println!("🚀 {} Store.save() calls per second", rate as u64);
    println!("");

    println!("🛁 Requesting all objects in random order...");
    let keys = store.keys();
    assert_eq!(keys.len(), COUNT);
    let start = Instant::now();
    for _ in 0..LOOPS {
        for hash in keys.iter() {
            assert!(store.load(&hash, &mut obj)?);
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    let rate = (COUNT * LOOPS) as f64 / elapsed;
    println!("🚀 {} Store.load() validated reads per second", rate as u64);
    println!("");

    println!("🛁 Reindexing objects...");
    let start = Instant::now();
    for _ in 0..LOOPS {
        store.reindex(&mut obj)?;
    }
    let elapsed = start.elapsed().as_secs_f64();
    let rate = (COUNT * LOOPS) as f64 / elapsed;
    println!("🚀 {} objects indexed plus validated per second", rate as u64);
    println!("");
    assert_eq!(store.len(), COUNT);

    println!("😎 Yes, Tub 🛁 is fast. 🚀");
    Ok(())
}
