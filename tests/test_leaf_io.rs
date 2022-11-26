use bathtub_db::base::*;
use bathtub_db::protocol::*;
use bathtub_db::leaf_io::*;
use bathtub_db::util::{random_small_object};

#[cfg(test)]

#[test]
fn test_tub_top() {
    let mut tt = TubTop::new();
    assert_eq!(tt.as_buf(), &[0_u8; HEADER_LEN]);
    assert_eq!(tt.hash(), [0_u8; TUB_HASH_LEN]);
    assert_eq!(tt.size(), 0);
    let obj = random_small_object();
    assert_eq!(tt.as_buf().len(), HEADER_LEN);
    tt.hash_next_leaf(&obj);
    assert_eq!(tt.as_buf().len(), HEADER_LEN + TUB_HASH_LEN);
    let info = hash_leaf(0, &obj);
    assert_eq!(tt.leaf_hash(0), info.hash);

    tt.finalize();    
    let root = hash(&obj);
    assert_eq!(tt.hash(), root.hash);
    assert_eq!(tt.size(), root.size);
    assert!(! tt.is_large());
    assert!(tt.is_small());

    tt.reset();
    assert_eq!(tt.size(), 0);
}

