//use crate::base::*;
use crate::util::*;
use crate::tree::*;


//Points to a root tree node
pub struct Commit {
    root: Tree,
    //author: TODO: Define how to represent authorship
    msg: String,
}

impl Commit {
    pub fn new(root: Tree, msg: String) -> Self {
        Self {
            root: root,
            msg: msg,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::*;
    use crate::helpers::TestTempDir;
    
    fn new_store() -> Store {
        const ROUNDS: u64 = 10_000;

        let (_tmp, mut store) = Store::new_tmp();
        store.reindex().unwrap();
        for _ in 0..ROUNDS {
            store.add_object(&random_object_id());
        }
        store
    }
    
    #[test]
    fn new_commit() {
        // Create db with objects
        let store = new_store();
        let keys = store.keys();
        
        //create a root tree node
        let mut tree = Tree::new();
        //let mut count: u64 = 0;
        for id in keys.iter() {
            tree.add(id);
        }
        
        //create commit object
        let msg: String = "Git Good, Rustling!".to_string();
        let _commit = Commit::new(tree, msg);
        
        
    }
}
