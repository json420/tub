use std::path::PathBuf;
use std::io;
use bathtub_db::dvcs::scan_tree;
use bathtub_db::store::Store;
use bathtub_db::dbase32::db32enc_str;


fn main() -> io::Result<()> {
    let (_tmp, mut store) = Store::new_tmp();
    let accum = scan_tree(&PathBuf::from("."))?;
    //let store = ts.into_store();
    println!("{}", accum.tree_objects.len());
    println!("{}", accum.files_info.len());

    for f in accum.files_info.iter() {
        let (hash, new) = store.import_file(f.open()?, f.size)?;
        assert_eq!(hash, f.hash);
    }
    for obj in accum.tree_objects.iter() {
        let (hash, new) = store.add_object(&obj)?;
    }
    store.reindex();
    Ok(())
}
