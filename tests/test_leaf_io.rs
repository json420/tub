use bathtub_db::base::*;
use bathtub_db::leaf_io::*;

#[cfg(test)]


#[test]
fn test_tub_buf2() {
    let tbuf = TubBuf::new();
    assert_eq!(tbuf.len(), 0);
}

#[test]
fn test_reindex_buf() {
    let mut rbuf = ReindexBuf::new();
    assert!(! rbuf.is_object());
    assert!(! rbuf.is_tombstone());
    assert_eq!(rbuf.size(), 0);
    assert_eq!(rbuf.as_mut_buf().len(), HEADER_LEN);
}

