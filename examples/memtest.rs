use bathtub_db::dbase32::encode;
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
    println!("hello");
    let mut index: HashMap<ObjectID, Entry> = HashMap::new();
    for i in 0..COUNT {
        let id = random_object_id();
        let entry = Entry {
            offset: random_u64(),
            size: random_u64(),
        };
        index.insert(id, entry);
    }
    println!("entering infinite loop, dude");
    loop {}
}
