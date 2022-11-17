use std::ffi::{OsString, OsStr};
use std::path::PathBuf;
use std::env;
use std::io;
use std::fs;
use std::process::exit;


use clap::{Args, ArgAction, Parser, Subcommand, ValueEnum};

use crate::store::{find_store, init_tree, Store};
use crate::importer::Scanner;


#[derive(Debug, Parser)]
#[command(name="tub")]
#[command(about="The most kickass DVCS of all?")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, action=ArgAction::SetTrue)]
    #[arg(help="Write buttloads of debuging stuffs to stderr")]
    verbose: bool,

    #[arg(short, long)]
    #[arg(help="Path to control directory (defaults to CWD)")]
    tub: Option<PathBuf>,
}


/*
impl Cli {
    pub fn get_tub(&self) -> io::Result<Store>
    {
        let target = dir_or_cwd(self.tub)?;
        find_store(&target)
    }
}
*/


#[derive(Debug, Subcommand)]
enum Commands {

    #[command(about = "Initalize a Bathtub DB repository")]
    Init {
        #[arg(help = "Target directory (defaults to CWD)")]
        target: Option<PathBuf>,
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

fn cmd_init(target: Option<PathBuf>) -> io::Result<()>
{
    let target = dir_or_cwd(target)?;
    if let Ok(store) = find_store(&target) {
        eprintln!("Store alread exists at {:?}", target);
        exit(42);
    }
    else if let Ok(store) = init_tree(&target) {
        eprintln!("created store at {:?}", store.path());
    }
    Ok(())
}

fn cmd_import(source: Option<PathBuf>) -> io::Result<()>
{
    let source = dir_or_cwd(source)?;
    let files = Scanner::scan_dir(&source);
    for src in files.iter() {
        println!("{:?}", src.0);
    }
    Ok(())
}


pub fn run() -> io::Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::Init {target} => {
            cmd_init(target)
        }
        Commands::Import {source} => {
            cmd_import(source)
        }
    }
}
