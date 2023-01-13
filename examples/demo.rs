use std::io;
use tub::chaos::{DefaultName, DefaultObject, DefaultStore};
use tub::inception::Fanout;
use tub::helpers::TestTempDir;
use tub::util::getrandom;


const COUNT: usize = 65536;

fn main() -> io::Result<()> {
    let tmp = TestTempDir::new();
    let file = tmp.create(&["some_file.store"]);
    let store = DefaultStore::new(file);
    let obj = DefaultObject::new();
    let mut fanout = Fanout::new(store, obj);
    let mut hash = DefaultName::new();
    let mut cont = DefaultName::new();
    for _ in 0..COUNT {
        getrandom(hash.as_mut_buf());
        assert!(fanout.get(&hash).unwrap().is_none());
        getrandom(cont.as_mut_buf());
        fanout.insert(hash.clone(), cont.clone()).unwrap();
        assert_eq!(fanout.get(&hash).unwrap().unwrap(), cont);
    }
    let (store, _obj) = fanout.into_inners();
    assert_eq!(store.len(), COUNT);
    Ok(())
}
