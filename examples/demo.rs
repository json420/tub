use std::path::PathBuf;
use std::io;
use bathtub_db::dvcs::TreeState;
use bathtub_db::store::Store;


fn main() -> io::Result<()> {
    let (_tmp, store) = Store::new_tmp();
    let ts = TreeState::new(store);
    ts.build_tree_state(&PathBuf::from("."))?;
    let store = ts.into_store();
    Ok(())
}
