use bathtub_db::base::*;
use bathtub_db::leaf_io::*;

#[cfg(test)]

#[test]
fn test_tub_top() {
    let tt = TubTop::new();
    let mut buf = tt.into_buf();
    assert_eq!(buf.len(), HEAD_LEN);
    buf.resize(0, 0);
    let tt = TubTop::new_with_buf(buf);
    assert_eq!(tt.len(), HEAD_LEN);

    let mut tt = TubTop::new();
    assert_eq!(tt.len(), HEAD_LEN);

    for size in [1, 2, 3, LEAF_SIZE - 1, LEAF_SIZE] {
        tt.resize_for_size(size);
        assert_eq!(tt.len(), HEAD_LEN);
        assert_eq!(tt.len(), get_preamble_size(size) as usize);

        tt.resize_for_size_plus_data(size);
        assert_eq!(tt.len(), HEAD_LEN + size as usize);
        assert_eq!(tt.len(), get_full_object_size(size) as usize);
    }

    for size in [LEAF_SIZE + 1, 2 * LEAF_SIZE - 1, 2 * LEAF_SIZE] {
        tt.resize_for_size(size);
        assert_eq!(tt.len(), HEAD_LEN + TUB_HASH_LEN);
        assert_eq!(tt.len(), get_preamble_size(size) as usize);
    }
}

