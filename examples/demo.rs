use std::path::PathBuf;
use std::io;
use bathtub_db::dvcs::{scan_tree, restore_tree, commit_tree};
use bathtub_db::store::Store;
use bathtub_db::dbase32::db32enc_str;


fn main() -> io::Result<()> {
    let mut store = Store::new(&PathBuf::from("/home/jderose/src/bathtub_db/.bathtub_db"))?;
    //let (_tmp, mut store) = Store::new_tmp();
    store.reindex();
    let root = commit_tree(&mut store, &PathBuf::from("/usr/share/doc"))?;
    restore_tree(&mut store, &root, &PathBuf::from("/tmp/foo"))?;
    /*
    let (root2, accum) = scan_tree(&PathBuf::from("/tmp/foo"))?;
    assert_eq!(root2, root);
    println!("{}", accum.trees.len());
    println!("{}", accum.files.len());
    store.reindex()?;
    */

    Ok(())
}
