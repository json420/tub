use bathtub_db::store::Store;

const GET_LOOPS: usize = 10;
const VERIFY: bool = true;

fn main() {
    println!("Hello");
    let mut store = Store::new("test.btdb");
    store.reindex(VERIFY);
    println!("Re-indexed {} objects", store.len());

    let keys = store.keys();
    for _ in 0..GET_LOOPS {
        for id in keys.iter() {
            store.get_object(id, VERIFY);
        }
    }
    println!("Called Store.get_object() {} times", GET_LOOPS * store.len());

}
