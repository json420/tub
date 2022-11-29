#[cfg(test)]

use std::io::prelude::*;
use std::fs::File;
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


fn mk_rand_obj_list(count: usize) -> Vec<RandObj> {
    let mut list: Vec<RandObj> = Vec::new();
    for _ in 0..count {
        list.push(mk_rand_obj());
    }
    list
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
    let (_tmp, mut store) = Store::new_tmp();
    store.add_object(&a.data).unwrap();
    store.add_object(&b.data).unwrap();
    store.delete_object(&a.hash).unwrap();
    store.add_object(&c.data).unwrap();
    store.reindex().unwrap();
    store.delete_object(&b.hash).unwrap();
    store.reindex().unwrap();
    assert_eq!(store.len(), 1);

    let mut pb = store.path();
    pb.push(PACKFILE);
    let mut file = File::options().append(true).open(&pb).unwrap();
    file.write_all(b"some extra junk").unwrap();
    store.reindex().unwrap();
}


#[test]
fn test_store_reindex() {
    let (_tmp, mut store) = Store::new_tmp();
    let count = 999;
    let list = mk_rand_obj_list(count);
    for robj in list.iter() {
        store.add_object(&robj.data).unwrap();
    }
    store.reindex().unwrap();
    assert_eq!(store.len(), count);
    let mut keys = store.keys();
    keys.sort();

    let mut del = Vec::new();
    let mut keep = Vec::new();
    for i in 0..count {
        if i % 3 == 0 {
            del.push(keys[i]);
        }
        else {
            keep.push(keys[i]);
        }
    }

    for hash in del.iter() {
        assert_eq!(store.delete_object(hash).unwrap(), true);
    }
    assert_eq!(store.len(), count * 2 / 3);
    for hash in del.iter() {
        assert!(store.get_object(hash, false).unwrap().is_none());
    }
    for hash in keep.iter() {
        assert!(store.get_object(hash, false).unwrap().is_some());
    }
    for hash in del.iter() {
        assert_eq!(store.delete_object(hash).unwrap(), false);
    }

    store.reindex().unwrap();
    assert_eq!(store.len(), count * 2 / 3);
    for hash in del.iter() {
        assert!(store.get_object(hash, false).unwrap().is_none());
    }
    for hash in keep.iter() {
        assert!(store.get_object(hash, false).unwrap().is_some());
    }
    for hash in del.iter() {
        assert_eq!(store.delete_object(hash).unwrap(), false);
    }
}

