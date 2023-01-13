use std::io;
use tub::chaos::DefaultObject;


fn main() -> io::Result<()> {
    let mut obj = DefaultObject::new();
    obj.clear();
    Ok(())
}
