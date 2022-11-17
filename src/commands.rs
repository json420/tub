use std::ffi::{OsString, OsStr};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};


#[derive(Debug, Parser)]
#[command(name = "tub")]
#[command(about = "The most kickass DVCS of all?")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}


#[derive(Debug, Subcommand)]
enum Commands {

    #[command(about = "Initalize a Bathtub DB repository")]
    Init {
        #[arg(help = "Target directory (defaults to current working directory)")]
        target: Option<PathBuf>,
    }
}



pub fn run() {
    let args = Cli::parse();
    match args.command {
        Commands::Init { target } => {
            println!("init {:?}", target);
        }
    }
}
