use std::io;
use std::time::Instant;
use tub::chaos::{DefaultName, DefaultObject, DefaultStore};
use tub::inception::Fanout;
use tub::helpers::TestTempDir;
use tub::util::getrandom;


const COUNT: usize = 1_000_000;
const LOOPS: usize = 10;

fn main() -> io::Result<()> {
    let tmp = TestTempDir::new();
    let file = tmp.create(&["some_file.store"]);
    let mut store = DefaultStore::new(file);
    let mut obj = DefaultObject::new();

    println!("🤔 Is Tub 🛁 fast? 🚀");
    println!("🛁 Saving {} random 1-256 byte sized objects...", COUNT);
    let start = Instant::now();
    for _ in 0..COUNT {
        obj.randomize(true);
        store.save(&mut obj)?;
    }
    let elapsed = start.elapsed().as_secs_f64();
    let rate = COUNT as f64 / elapsed;
    println!("🚀 {} saves per second", rate as u64);

    println!("🛁 Loading {} objects, looping {} times...", COUNT, LOOPS);
    let keys = store.keys();
    let start = Instant::now();
    for _ in 0..LOOPS {
        for hash in keys.iter() {
            store.load(&hash, &mut obj);
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    let rate = (COUNT * LOOPS) as f64 / elapsed;
    println!("🚀 {} loads per second", rate as u64);
    println!("😎 Yes, Tub 🛁 is fast. 🚀");
    Ok(())
}
