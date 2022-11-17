use crate::base::*;
use crate::util::*;
use crate::branchentry::*;



//Branch: (blockchain of branch entries, fork and merge semantics)

pub struct Branch {
    chain: Vec<BranchEntry>,
    //source_branch: Option<& Branch>,
    //source_entry: Option<BranchEntry>,
}

impl Branch {
    pub fn new() -> Self {
        Self {
            chain: vec![],
            //source_branch: None,
            //source_entry: None,
        }
    }
    
    pub fn new2(branch: &Branch) -> Self {
        Self {
            chain: vec![],
            //source_branch: Some(branch),
            //source_entry: Some((Some(branch).unwrap()).get_head()),
        }
    }
    
    //pub fn get_head(&self) -> BranchEntry {
    //    let head: &BranchEntry = &*(self.chain.last().unwrap());
    //    *head
    //}
}
