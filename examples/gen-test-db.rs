use bathtub_db::store::Store;
use bathtub_db::util::{getrandom, random_u16};

fn main() {
    let mut store = Store::new("test.btdb");

    let mut buf = vec![0_u8; 4096];
    let mut total: usize = 0;
    for i in 0..100_000 {
        let size = random_u16() as usize;
        total += size;
        buf.resize(size, 0);
        let s = &mut buf[0..size];
        getrandom(s);
        store.add_object(s);
        println!("{}\t{}", i, size);
    }
    
    println!("{}", total);
}


