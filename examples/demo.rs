use std::path::PathBuf;
use std::io;
use bathtub_db::dvcs::{scan_tree, restore_tree};
use bathtub_db::store::Store;
use bathtub_db::dbase32::db32enc_str;


fn main() -> io::Result<()> {
    let (_tmp, mut store) = Store::new_tmp();
    let (root, accum) = scan_tree(&PathBuf::from("."))?;
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
    store.reindex()?;
    restore_tree(&root, &mut store, &PathBuf::from("/tmp/foo"));
    Ok(())
}
