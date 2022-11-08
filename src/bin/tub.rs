use std::env;

fn main()
{
    println!("{:?}", env::current_dir());
    for a in env::args() {
        println!("{}", a);
    }
}
