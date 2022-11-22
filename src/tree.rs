use crate::base::*;
use crate::util::*;
use std::cmp::Ordering;

//#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct AOPair {
    id: [u8; TUB_ID_LEN],
    obj_id: [u8; TUB_HASH_LEN],
}

impl PartialEq for AOPair {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for AOPair { }

impl PartialOrd for AOPair {
    fn partial_cmp (&self, other: &Self) -> Option<Ordering> {
        Some(self.id.cmp(&(other.id)))
    }
}

impl Ord for AOPair {
    fn cmp (&self, other: &Self) -> Ordering {
        (self.id).cmp(&(other.id))
    }
}

pub struct Tree {
    ids: Vec<AOPair>,
    cur: usize, // iterate
}

impl Tree {
    pub fn new() -> Self {
        Self {
            ids: vec![],
            cur: 0,
        }
    }
    
    pub fn add(&mut self, obj_id: &[u8; 30]) {
        //self.ids.push(AOPair{id: random_id(), obj_id: *obj_id});
        //absid: getrandom
        //util.randomid
        let id = random_id();
        match self.ids.binary_search(&AOPair{id: id, obj_id: *obj_id}) {
            Ok(_) => {},
            Err(pos) => self.ids.insert(pos, AOPair{id: id, obj_id: *obj_id}),
        }
    }
    
    //this is used for testing
    pub fn add_with_abs_id(&mut self, abs_id: &[u8; 15], obj_id: &[u8; 30]) {
        let id = abs_id.clone();
        match self.ids.binary_search(&AOPair{id: id, obj_id: *obj_id}) {
            Ok(_) => {},
            Err(pos) => {self.ids.insert(pos, AOPair{id: id, obj_id: *obj_id})},
        }
    }
    
    pub fn read_next_id(&mut self) -> TubId {
        let r = self.ids[self.cur].id;
        self.cur += 1;
        r
    }
    
    pub fn get_object_id(&mut self, abstract_id: TubId) -> TubHash {
        let len: f64 = self.ids.len() as f64;
        let fraction: f64 = abstract_id[0] as f64 * len / 256.0;
        let mut i = fraction.floor() as usize;
        
        while abstract_id != self.ids[i].id {
            if abstract_id < self.ids[i].id {
                i -= 1;
            }
            else if abstract_id > self.ids[i].id {
                i += 1;
            }
        }
        self.ids[i].obj_id
    }
    
    pub fn get_tree_object(&mut self) -> Vec<u8> {
        let mut obj: Vec<u8> = Vec::with_capacity(self.ids.len()*(TUB_ID_LEN+TUB_HASH_LEN));
        obj.push(0u8);
        for el in 0..self.ids.len() {
            obj.extend_from_slice(&self.ids[el].id);
            obj.extend_from_slice(&self.ids[el].obj_id);
        }
        obj
    }
    
}


//same encoding but only include the keys that have changed
impl Iterator for Tree {
    type Item = TubId;
    
    fn next(&mut self) -> Option<Self::Item> {
        
        self.cur += 1;
        if self.cur <= self.ids.len() {
            Some(self.ids[self.cur-1].id as TubId)
        }
        else { None }
    }
    
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.ids[n].id as TubId)
    }

}

//each commit include the differences
//if you branch from it the first entry would contain all of them
#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::*;

    #[test]
    fn it_works() {
        let mut tree = Tree::new();
        tree.add(&[0u8; 30]);
        let _ret = tree.read_next_id();
        //assert_eq!(ret, [0u8; 15]);
    }
    
    #[test]
    fn iterable() {
        let mut tree = Tree::new();
        let oid1 = random_hash();
        tree.add(&oid1);
        let aid1 = tree.read_next_id();
        tree.cur = 0;
        
        for id in tree.into_iter() {
            assert_eq!(id, aid1);
        }
        
    }
    
    #[test]
    fn get_tree_obj() {
        let mut tree = Tree::new();
        let mut oid1 = [0u8; 30];  //use util.random_hash()
        let mut oid2 = [1u8; 30];
        tree.add(&oid1);
        
        tree.add(&oid2);
        
        let aid1 = tree.read_next_id();
        let aid2 = tree.read_next_id();
        
        let ret = tree.get_tree_object();
        if ret[18] == 1 {
            let tmpoid = oid1;
            oid1 = oid2;
            oid2 = tmpoid;
        }
        
        let mut right = [0u8; 91];
        right[0] = 0;
        right[1..16].copy_from_slice(&aid1);
        right[16..46].copy_from_slice(oid1.as_slice());
        right[46..61].copy_from_slice(&aid2);
        right[61..91].copy_from_slice(oid2.as_slice());
        
        assert_eq!(ret, right);
    }
    
    #[test]
    fn add_db() {
        let (_tmp, mut store) = Store::new_tmp();
        store.reindex().unwrap();

        const ROUNDS: u64 = 10_000;

        for _id in 0..ROUNDS {
            store.add_object(&random_hash());
        }
        
        let keys = store.keys();
        
        let mut tree = Tree::new();
        let mut count: u64 = 0;
        for id in keys.iter() {
            tree.add(id);
            count += 1;
        }
        assert_eq!(count, ROUNDS);
        
        let mut prevabs: [u8; TUB_ID_LEN] = [0u8; TUB_ID_LEN];
        count = 0;
        for _id in 0..ROUNDS {
            let abs = tree.read_next_id();
            if abs > prevabs {
                assert_eq!(abs, abs);
            }
            else {
                assert_eq!((abs, count), (prevabs, count));
            }
            prevabs = abs;
            count += 1;
        }
        
    }
}
