use bathtub_db::store::Store;

const GET_LOOPS: usize = 5;

fn main() {
    println!("Hello");
    let mut store = Store::new("test.btdb");
    store.reindex(false);
    println!("Re-indexed {} objects", store.len());

    let keys = store.keys();
    for _ in 0..GET_LOOPS {
        for id in keys.iter() {
            store.get_object(id);
        }
    }
    println!("Called Store.get_object() {} times", GET_LOOPS * store.len());

}
