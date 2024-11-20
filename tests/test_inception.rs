use tub::inception::*;

#[cfg(test)]
#[test]
fn test_leaf_hashes() {
    let lh: LeafHashes<30> = LeafHashes::new();
    let mut buf = Vec::new();
    lh.serialize(&mut buf);
    assert_eq!(buf, vec![0; 8]);
}
