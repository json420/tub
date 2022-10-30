use bathtub_db::util::*;
use bathtub_db::store::Store;
use std::time;

const COUNT: usize = 100_000;

fn main() {
    let objects = bulk_random_small_objects(COUNT);

    let (tmp, mut store) = Store::new_tmp();
    let start = time::Instant::now();
    for obj in objects.iter() {
        store.add_object(obj);
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!("Add {} objects", store.len());
    println!("Took {} seconds", elapsed);
    println!("{} objects/second add rate",
        (store.len() as f64 / elapsed) as u64
    );

}
