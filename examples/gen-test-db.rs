use bathtub_db::store::Store;
use bathtub_db::util::{getrandom, random_u16};

const COUNT: usize = 100_000;
static NAME: &str  = "test.btdb";

fn main() {
    let mut store = Store::new(NAME);
    store.reindex(true);
    println!("Re-indexed {} existing objects", store.len());
    let mut buf = vec![0_u8; 4096];
    let mut total: usize = 0;
    for i in 0..COUNT {
        let size = random_u16(16) as usize;  // Cuz we want size >= 16 bytes
        total += size;
        buf.resize(size, 0);
        let s = &mut buf[0..size];
        getrandom(s);
        let (id, new) = store.add_object(s);
        assert!(new);  // All objects should be unique cuz size >= 16 bytes
        
    }
    //store.file.sync_all();
    println!("Added {} new objects, {} bytes in total", COUNT, total);
}


