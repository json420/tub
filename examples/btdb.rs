use bathtub_db::base::*;
use bathtub_db::util::*;
use bathtub_db::store::Store;
use bathtub_db::importer::Scanner;
use std::time;
use std::fs;
use std::io::prelude::*;


const COUNT: usize = 100_000;

fn main() {
    //let objects = bulk_random_small_objects(COUNT);

    let (tmp, mut store) = Store::new_tmp();
    /*
    let start = time::Instant::now();
    for obj in objects.iter() {
        store.add_object(obj);
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!("Add {} objects", store.len());
    println!("Took {} seconds", elapsed);
    println!("{} objects/second add rate",
        (store.len() as f64 / elapsed) as u64
    );
    */

    let sr = Scanner::scan_dir("/usr/share");
    let mut buf: Vec<u8> = Vec::with_capacity(16 * 1024);
    let start = time::Instant::now();
    for f in sr.iter() {
        //let m = fs::metadata(p).unwrap();
        if let Ok(mut file) = fs::File::open(&f.0) {
            buf.clear();
            let size = file.read_to_end(&mut buf).unwrap();
            assert_eq!(size as u64, f.1);
            store.add_object(&buf);
        }
        else {
            println!("{:?} {}", f.0, f.1);
        }
        
    }
    //store.sync_data();
    let elapsed = start.elapsed().as_secs_f64();
    println!("Add {} objects", store.len());
    println!("Took {} seconds", elapsed);
    println!("{} objects/second add rate",
        (store.len() as f64 / elapsed) as u64
    );

}
