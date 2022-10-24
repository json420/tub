use bathtub_db::store::Store;
use bathtub_db::util::{getrandom, random_u16};
use bathtub_db::dbase32::encode;

const COUNT: usize = 50_000;

fn main() {
    let mut store = Store::new("test.btdb");
    let mut buf = vec![0_u8; 4096];
    let mut total: usize = 0;
    for i in 0..50_000 {
        let size = random_u16() as usize;
        total += size;
        buf.resize(size, 0);
        let s = &mut buf[0..size];
        getrandom(s);
        let (id, _entry) = store.add_object(s);
        println!("{}\t{}\t{}", encode(&id), i, size);
    }
    println!("{} objects, {} bytes total", COUNT, total);
}


