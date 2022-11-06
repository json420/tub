use bathtub_db::dbase32::Name2Iter;
use bathtub_db::store::init_store_layout;
use openat;


fn main() {
    let dir = openat::Dir::open(".").unwrap();
    init_store_layout(&dir).unwrap();
}
