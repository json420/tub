//use bathtub_db::commands::{get_args, run};

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{arg, Command};


fn cli() -> Command
{
    Command::new("tub")
        .about("Super weird and awesome DVCS")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("init")
                .about("Initialized a BathtubDB repository")
        )
        .subcommand(
            Command::new("commit")
                .about("Commit stuff and junk")
        )
        .subcommand(
            Command::new("import")
                .about("Import files into object store")
        )
        
}



fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("init", sub_matches)) => {
            println!("running init");
        }
        
        
        _ => unreachable!(),
    }
}
