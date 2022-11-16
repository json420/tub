
use std::collections::{BTreeMap, VecDeque};
use std::env;


pub type CmdArgs = VecDeque<String>;

/// Collect `env::args()` into a `VecDeque`.
pub fn get_args() -> CmdArgs
{
    let mut args: CmdArgs = env::args().collect();
    args.pop_front().unwrap();
    args
}


trait Command {
    fn name(&self) -> String;
    fn run(&self, args: &CmdArgs);
}

pub type CmdType = Box<dyn Command>;


struct InitCmd {

}
impl Command for InitCmd {
    fn name(&self) -> String {
        "init".to_string()
    }

    fn run(&self, args: &CmdArgs) {
        println!("runnig init");
    }
}

struct ImportCmd {

}
impl Command for ImportCmd {
    fn name(&self) -> String {
        "import".to_string()
    }

    fn run(&self, args: &CmdArgs) {
        println!("runnig import");
    }
}


struct Dispatcher {
    map: BTreeMap<String, CmdType>,
}

impl Dispatcher {
    fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    fn add(&mut self, cmd: CmdType) {
        self.map.insert(cmd.name(), cmd);
    }

    fn run(&self, args: &mut CmdArgs) {
        if let Some(name) = args.pop_front() {
            if let Some(cmd) = self.map.get(&name) {
                cmd.run(args);
            }
            else {
                eprintln!("Unknown command: {:?}", name);
            }
        }
        else {
            eprintln!("Available commands:");
            for cmd in self.map.values() {
                eprintln!("  {}", cmd.name());
            }
        }
    }

}

fn build_dispatcher() -> Dispatcher {
    let mut d = Dispatcher::new();
    d.add(Box::new( ImportCmd {} ));
    d.add(Box::new( InitCmd {} ));
    //d.add(Box::new( ImportCmd {} ));
    d
}



pub fn run(args: &mut CmdArgs) {
    let dispatcher = build_dispatcher();
    dispatcher.run(args);
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
    }
}

