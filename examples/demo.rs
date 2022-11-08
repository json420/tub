use bathtub_db::dbase32::{Name2Iter, random_id};
use bathtub_db::util::random_object_id;
use bathtub_db::store::Store;

fn main() {
    for name in Name2Iter::new() {
        println!("{}", name);
    }
    let (tmp, store) = Store::new_tmp();
    for _i in 0..10 {
        let rid = random_object_id();
        println!("{}", store.object_path(&rid).display());
        println!("{}", store.partial_path(&rid).display());
    }
    println!("{}", random_id());
}
