use bathtub_db::store::Store;
use bathtub_db::util::{getrandom, random_u16};
use bathtub_db::dbase32::encode;

const COUNT: usize = 100_000;
static NAME: &str  = "test.btdb";

fn main() {
    let mut store = Store::new(NAME);
    store.reindex(true);
    println!("Re-indexed {} existing objects", store.len());
    let mut buf = vec![0_u8; 4096];
    let mut total: usize = 0;
    for i in 0..COUNT {
        let size = random_u16(16) as usize;
        total += size;
        buf.resize(size, 0);
        let s = &mut buf[0..size];
        getrandom(s);
        let (id, new) = store.add_object(s);
        if !new {
            println!("duplicate {} {} {:?}", i, size, id);
        }
        //println!("{}\t{}\t{}", encode(&id), i, size);
    }
    //store.file.sync_all();
    println!("Add {} objects, {} bytes total", COUNT, total);
}


