use bathtub_db::store::LeafInfoIter;
use bathtub_db::base::LEAF_SIZE;

fn main() {
    let size = LEAF_SIZE * 10 + 11;
    for info in LeafInfoIter::new(size, 0) {
        println!("{:?}", info);
    }
    println!("done");
}
