//! Command line argument parsing and dispatching.

use std::env;
use std::collections::{HashMap, VecDeque};


pub type CmdArgs = VecDeque<String>;
pub type CmdFn = fn(args: &CmdArgs) -> bool;
pub type CmdMap = HashMap<String, CmdFn>;


/// Collect `env::args()` into a `VecDeque`.
pub fn get_args() -> CmdArgs
{
    let mut args: CmdArgs = env::args().collect();
    args.pop_front().unwrap();
    args
}


pub struct Dispatcher {
    map: CmdMap,
}

impl Dispatcher {
    fn new() -> Self {
        Self {map: HashMap::new()}
    }

    fn add(&mut self, name: &str, cmd: CmdFn) {
        self.map.insert(name.to_string(), cmd);
    }

    fn run(&self, args: &mut CmdArgs) {
        if let Some(name) = args.pop_front() {
            if let Some(cmd) = self.map.get(&name) {
                cmd(args);
            }
        }

    }
}

pub fn build_dispatcher() -> Dispatcher {
    let mut dis = Dispatcher::new();
    dis.add("init", cmd_init);
    dis   
}

pub fn run(args: &mut CmdArgs) {
    let dispatcher = build_dispatcher();
    dispatcher.run(args);
}


fn cmd_init(args: &CmdArgs) -> bool {
    println!("yo from init");
    true
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_args() {
        get_args();
    }

    #[test]
    fn test_dispatcher() {
        let mut d = Dispatcher::new();
        d.add("init", cmd_init);
    }
}

