use bathtub_db::dbase32::Name2Iter;
use bathtub_db::util::random_object_id;
use bathtub_db::store::Store;

fn main() {
    for name in Name2Iter::new() {
        println!("{}", name);
    }
    let (tmp, store) = Store::new_tmp();
    for _i in 0..10 {
        println!("{}", store.object_path(&random_object_id()).display());
    }
}
