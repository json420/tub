use bathtub_db::util::{random_object_id, random_u64};
use bathtub_db::base::*;
use std::time;
use std::collections::HashMap;

const COUNT: usize = 1_000_000;
const LOOPS: usize = 10;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Entry {
    offset: OffsetSize,
    size: ObjectSize,
}

fn main() {
    println!("Inserting {} random entries into index...", COUNT);
    let mut index: HashMap<ObjectID, Entry> = HashMap::new();
    let start = time::Instant::now(); 
    for _i in 0..COUNT {
        let id = random_object_id();
        let entry = Entry {
            offset: random_u64(),
            size: random_u64(),
        };
        index.insert(id, entry);
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!("  Added {} objects", index.len());
    println!("  Took {} seconds", elapsed);
    println!("  {} objects/second indexing rate",
        (index.len() as f64 / elapsed) as u64
    );

    let keys = Vec::from_iter(index.keys().cloned());
    let start = time::Instant::now();
    for _ in 0..LOOPS {
        for id in keys.iter() {
            index.get(id).unwrap();
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!("  Got {} objects", index.len());
    println!("  Took {} seconds", elapsed);
    println!("  {} objects/second get rate",
        (index.len() as f64 / elapsed) as u64
    );
}
