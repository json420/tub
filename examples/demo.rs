#![feature(mutex_unlock)]

use std::sync::Mutex;

use bathtub_db::store::Store;
use bathtub_db::dbase32::encode;

fn main() {
    println!("hello");
    let mut store = Store::new("test.btdb");
    store.reindex(false);
/*
    let index = store.index.lock().unwrap();
    let keys = Vec::from_iter(index.keys().cloned());
    Mutex::unlock(index);

    let mut i = 0;
    for id in keys.iter() {
        //println!("{} {}", encode(id), i);
        store.get_object(id);
        i += 1;
    }
*/
}
