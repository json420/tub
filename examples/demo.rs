use bathtub_db::store::Store;


fn main() {
    println!("hello");
    let mut store = Store::new("test.btdb");
    store.reindex();
}
