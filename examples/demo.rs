use std::path::PathBuf;
use std::io;
use bathtub_db::dvcs::build_tree_state;


fn main() -> io::Result<()> {
    build_tree_state(&PathBuf::from("."), 0);
    Ok(())
}
