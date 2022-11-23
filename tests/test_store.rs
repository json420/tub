use bathtub_db::store::Store;
use bathtub_db::util::random_hash;

#[cfg(test)]

#[test]
fn test_get_object() {
    let (_tmp, mut store) = Store::new_tmp();
    store.reindex();
    let ch = random_hash();
    assert!(store.get_object(&ch, false).is_ok());
}
