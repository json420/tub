use std::path::PathBuf;
use std::io;
use bathtub_db::dvcs::{scan_tree, restore_tree};
use bathtub_db::store::Store;
use bathtub_db::dbase32::db32enc_str;


fn main() -> io::Result<()> {
    //let mut store = Store::new(&PathBuf::from("/tmp/bar/.bathtub_db"))?;
    let (_tmp, mut store) = Store::new_tmp();
    //store.reindex();
    let (root, accum) = scan_tree(&PathBuf::from("/usr/share/doc"))?;
    //let store = ts.into_store();
    println!("{}", accum.trees.len());
    println!("{}", accum.files.len());

    for f in accum.files.iter() {
        let (hash, new) = store.import_file(f.open()?, f.size)?;
        assert_eq!(hash, f.hash);
    }
    for t in accum.trees.iter() {
        let (hash, new) = store.add_object(&t.data)?;
        assert_eq!(hash, t.hash);
    }
    //
    restore_tree(&root, &mut store, &PathBuf::from("/tmp/foo"))?;

    let (root2, accum2) = scan_tree(&PathBuf::from("/tmp/foo"))?;
    assert_eq!(root2, root);
    println!("{}", accum.trees.len());
    println!("{}", accum.files.len());
    store.reindex()?;

    Ok(())
}
