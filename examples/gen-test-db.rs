use std::time;
use bathtub_db::store::Store;
use bathtub_db::util::{getrandom, random_u16};

const COUNT: usize = 100_000;
static NAME: &str  = "test.btdb";
const VERIFY: bool = false;

fn main() {
    let mut store = Store::new("test.btdb");
    println!("Reindexing objects (VERIFY={})", VERIFY);

    let start = time::Instant::now();
    store.reindex(VERIFY);
    let elapsed = start.elapsed().as_secs_f64();
    println!("  Indexed {} existing objects", store.len());
    println!("  Took {} seconds", elapsed);
    println!("  {} objects/second indexing rate",
        (store.len() as f64 / elapsed) as u64
    );

    println!("Adding {} random objects", COUNT);
    let mut buf = vec![0_u8; 4096];
    let mut total: usize = 0;
    let start = time::Instant::now();
    for i in 0..COUNT {
        let size = random_u16(16) as usize;  // Cuz we want size >= 16 bytes
        total += size;
        buf.resize(size, 0);
        let s = &mut buf[0..size];
        getrandom(s);
        let (id, new) = store.add_object(s);
        assert!(new);  // All objects should be unique cuz size >= 16 bytes
        
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!("  Added {} new objects", COUNT);
    println!("  Took {} seconds", elapsed);
    println!("  {} objects/second add rate",
        (COUNT as f64 / elapsed) as u64
    );

    //store.file.sync_all();
    println!("Added {} new objects, {} bytes in total", COUNT, total);
}


