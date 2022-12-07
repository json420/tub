use std::path::PathBuf;
use std::io;
use bathtub_db::dvcs::TreeState;
use bathtub_db::store::Store;
use bathtub_db::dbase32::db32enc_str;


fn main() -> io::Result<()> {
    let (_tmp, store) = Store::new_tmp();
    let ts = TreeState::new(store);
    if let Some(hash) = ts.build_tree_state(&PathBuf::from("."))? {
        println!("root: {}", db32enc_str(&hash));
    }
    //let store = ts.into_store();
    Ok(())
}
