#[cfg(test)]
use std::fs;
use tub::chaos::{Name, Object, Store};
use tub::helpers::TestTempDir;
use tub::protocol::Blake3;

#[test]
fn test_roundtrip() {
    let tmp = TestTempDir::new();
    let pb = tmp.build(&["some_file"]);
    let file = fs::File::options()
        .read(true)
        .append(true)
        .create_new(true)
        .open(&pb)
        .unwrap();
    let mut store: Store<Blake3, 30> = Store::new(file);
    let mut obj: Object<Blake3, 30> = Object::new();
    let mut objects: Vec<(Name<30>, Vec<u8>)> = Vec::new();
    for _ in 0..2048 {
        obj.randomize(true);
        objects.push((obj.hash(), Vec::from(obj.as_buf())));
        store.save(&obj).unwrap();
    }
    for (hash, buf) in objects.iter() {
        store.load(&hash, &mut obj).unwrap();
        assert_eq!(obj.as_mut_vec(), buf);
    }
}
