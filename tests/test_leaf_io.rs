use bathtub_db::base::*;
use bathtub_db::leaf_io::*;

#[cfg(test)]


#[test]
fn test_tub_buf2() {
    let mut tbuf = TubBuf2::new();
    assert_eq!(tbuf.len(), 0);
}

