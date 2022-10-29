use std::time;
use bathtub_db::store::Store;
use bathtub_db::dbase32::db32enc_str;

const GET_LOOPS: usize = 10;
//const VERIFY: bool = true;
const VERIFY: bool = false;

fn main() {
    let mut store = Store::new("test.btdb", None);
    println!("Reindexing objects (VERIFY={})", VERIFY);

    let start = time::Instant::now();
    store.reindex(VERIFY);
    let elapsed = start.elapsed().as_secs_f64();
    println!("  Indexed {} objects", store.len());
    println!("  Took {} seconds", elapsed);
    println!("  {} objects/second indexing rate",
        (store.len() as f64 / elapsed) as u64
    );

    println!("Requesting all objects {} times (VERIFY={})", GET_LOOPS, VERIFY);
    let keys = store.keys();
    let start = time::Instant::now();
    for _ in 0..GET_LOOPS {
        for id in keys.iter() {
            store.get_object(id, VERIFY);
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    let count = GET_LOOPS * store.len();
    println!("  Reqested {} objects", count);
    println!("  Took {} seconds", elapsed);
    println!("  {} requests/second", (count as f64 / elapsed) as u64);

    for id in keys[0..1000].iter() {
        //println!("{}", db32enc_str(id));
        //store.delete_object(id);
    }

}
