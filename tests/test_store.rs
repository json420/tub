use bathtub_db::store::Store;
use bathtub_db::util::{random_hash, random_small_object};

#[cfg(test)]

#[test]
fn test_get_object() {
    let (_tmp, mut store) = Store::new_tmp();
    store.reindex().unwrap();
    let ch = random_hash();
    assert!(store.get_object(&ch, false).is_ok());
}

#[test]
fn test_get_object_new() {
    return;  // FIXME
    let (_tmp, mut store) = Store::new_tmp();
    let rch = random_hash();

    let mut buf: Vec<u8> = Vec::new();
    assert_eq!(buf.len(), 0);
    assert_eq!(store.get_object_new(&rch, &mut buf).unwrap(), false);
    assert_eq!(buf.len(), 0);

    let obj = random_small_object();
    let (top, new) = store.add_object(&obj).unwrap();
    assert!(new);
    assert_eq!(top.size(), obj.len() as u64);

    assert_eq!(store.get_object_new(&top.hash(), &mut buf).unwrap(), true);
    assert_eq!(buf.len(), obj.len());
    assert_eq!(buf, obj);
}
