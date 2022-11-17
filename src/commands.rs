use std::ffi::{OsString, OsStr};
use std::path::PathBuf;
use std::env;
use std::io;
use std::fs;
use std::process::exit;


use clap::{Args, ArgAction, Parser, Subcommand, ValueEnum};

use crate::store::init_tree;


#[derive(Debug, Parser)]
#[command(name="tub")]
#[command(about="The most kickass DVCS of all?")]
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

        #[arg(short, long, action=ArgAction::SetTrue)]
        #[arg(help="Write buttloads of debuging stuffs to stderr")]
        verbose: bool,
    },

    #[command(about = "Recursively import files from directory")]
    Import {
        #[arg(help = "Source directory (defaults to current working directory)")]
        source: Option<PathBuf>,
    },
}


fn dir_or_cwd(target: Option<PathBuf>) -> io::Result<PathBuf>
{
    let mut pb = match target {
        Some(dir) => dir,
        None => env::current_dir()?,
    };
    if ! pb.is_dir() {
        eprintln!("Not a directory: {:?}", pb);
        exit(42);
    }
    Ok(pb.canonicalize()?)
}


pub fn run() -> io::Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::Init {target,  verbose} => {
            //println!("init {:?} {:?}", target, verbose);
            let mut pb = dir_or_cwd(target)?;
            eprintln!("init {:?}", pb);
            init_tree(&mut pb)?;
        }
        Commands::Import {source} => {
            let pb = dir_or_cwd(source)?;
            eprintln!("import {:?}", pb);
        }
    }
    Ok(())
}
