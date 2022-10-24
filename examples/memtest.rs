use bathtub_db::util::{random_object_id, random_u64};
use bathtub_db::base::*;
use std::collections::HashMap;

const COUNT: usize = 10_000_000;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Entry {
    offset: OffsetSize,
    size: ObjectSize,
}

fn main() {
    println!("Inserting {} random entries into index...", COUNT);
    let mut index: HashMap<ObjectID, Entry> = HashMap::new();
    for _i in 0..COUNT {
        let id = random_object_id();
        let entry = Entry {
            offset: random_u64(),
            size: random_u64(),
        };
        index.insert(id, entry);
    }
    println!("Entering the infinite loop, dude!");
    loop {}
}
