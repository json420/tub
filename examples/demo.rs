use bathtub_db::dbase32::Name2Iter;

fn main() {
    for name in Name2Iter::new() {
        println!("{}", name);
    }
}
