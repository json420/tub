#[cfg(test)]

use bathtub_db::base::*;
use bathtub_db::store::Store;
use bathtub_db::leaf_io::TubTop;
use bathtub_db::util::{random_hash, random_small_object};

struct RandObj {
    data: Vec<u8>,
    hash: TubHash,
}

fn mk_rand_obj() -> RandObj {
    let data = random_small_object();
    let mut tt = TubTop::new();
    let hash = tt.hash_data(&data);
    RandObj {data: data, hash: hash}
}


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


#[test]
fn test_store_roundtrip() {
    let a = mk_rand_obj();
    let b = mk_rand_obj();
    let c = mk_rand_obj();

    // Make sure reindex correctly adjusts offset when tombstones are found
    let (tmp, mut store) = Store::new_tmp();
    store.add_object(&a.data).unwrap();
    store.add_object(&b.data).unwrap();
    store.delete_object(&a.hash).unwrap();
    store.add_object(&c.data).unwrap();
    store.reindex();
}

