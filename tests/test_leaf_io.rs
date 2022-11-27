use bathtub_db::base::*;
use bathtub_db::protocol::*;
use bathtub_db::leaf_io::*;
use bathtub_db::util::{random_small_object};

#[cfg(test)]

#[test]
fn test_tub_top() {
    let tt = TubTop::new();
    let buf = tt.into_buf();
    assert_eq!(buf.len(), HEAD_LEN);
    let tt = TubTop::new_with_buf(buf);

    let tt = TubTop::new();
    assert_eq!(tt.len(), HEAD_LEN);
}

