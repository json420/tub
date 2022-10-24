use crate::base::*;
use crate::util::*;

struct AOPair {
    id: [u8; ABSTRACT_ID_LEN],
    obj_id: [u8; OBJECT_ID_LEN],
}

struct Tree {
    ids: Vec<AOPair>,
    cur: usize,
}

impl Tree {
    fn new() -> Self {
        Self {
            ids: vec![],
            cur: 0,
        }
    }
    
    fn add(&mut self) {
        self.ids.push(AOPair{id: random_id(), obj_id: [0u8; 30]});
        //absid: getrandom
        //util.randomid
    }
    
    fn read_next_id(&mut self) -> AbstractID {
        let r = self.ids[self.cur].id;
        self.cur += 1;
        r
    }
}


//same encoding but only include the keys that have changed


//each commit include the differences
//if you branch from it the first entry would contain all of them
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let mut tree = Tree::new();
        tree.add();
        let ret = tree.read_next_id();
        //assert_eq!(ret, [0u8; 15]);
    }
    
    //fn add_db() {
    //    let 
    //}
}
