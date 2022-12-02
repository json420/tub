use bathtub_db::base::*;
use bathtub_db::leaf_io::*;

#[cfg(test)]


#[test]
fn test_tub_buf2() {
    let mut tbuf = TubBuf::new();
    assert_eq!(tbuf.len(), 0);
}

#[test]
fn test_reindex_buf() {
    let mut rbuf = ReindexBuf::new();
    //assert!(! rbuf.is_object());
    //assert!(! rbuf.is_tombstone());
}

