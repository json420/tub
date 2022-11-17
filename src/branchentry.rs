use crate::base::*;
use crate::util::*;
use crate::commit::*;

//BranchEntry: Sing and timestamp commits, increasing counter

//committer recorded here


pub struct BranchEntry {
    commit: Commit,
    //committer: TODO: Define how to represent committer
    revision: u64,
}

impl BranchEntry {
    pub fn new(commit: Commit, revision: u64) -> Self {
        Self {
            commit: commit,
            revision: revision,
        }
    }
}
