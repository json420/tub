#![feature(mutex_unlock)]

use std::sync::Mutex;

use bathtub_db::store::Store;
use bathtub_db::dbase32::encode;
use bathtub_db::base::*;

fn main() {
    println!("hello");
    let mut store = Store::new("test.btdb");
    store.reindex(false);

    let index = store.index.lock().unwrap();
    let keys = Vec::from_iter(index.keys().cloned());
    Mutex::unlock(index);

    for id in keys.iter() {
        println!("{}", encode(id));
        store.get_object(id);
    }
}
