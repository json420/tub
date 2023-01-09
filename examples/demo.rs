use std::io;
use std::io::prelude::*;
use zstd::stream::{Encoder, Decoder};
use tub::chaos::DefaultObject;


fn main() -> io::Result<()> {
    let mut obj = DefaultObject::new();
    obj.clear();
    let mut enc: Encoder<'static, DefaultObject> = Encoder::new(obj, 0)?;
    for i in 0..=255 {
        enc.write(b"hello world how are you today");
    }
    Ok(())
}
