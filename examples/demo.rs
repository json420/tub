use std::io;
use std::io::prelude::*;
use zstd::stream::{Encoder, Decoder};
use tub::chaos::DefaultObject;


fn main() -> io::Result<()> {
    let mut obj = DefaultObject::new();
    obj.clear();
    Ok(())
}
