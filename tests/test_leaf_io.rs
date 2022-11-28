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

    let tt = TubTop::new();
    assert_eq!(tt.len(), HEAD_LEN);
}

