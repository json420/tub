use std::ffi::{OsString, OsStr};
use std::path::PathBuf;
use std::env;
use std::io::prelude::*;
use std::io;
use std::fs;
use std::process::exit;


use clap::{Args, ArgAction, Parser, Subcommand, ValueEnum};

use crate::store::{find_store, init_tree, Store};
use crate::importer::Scanner;
use crate::dbase32::db32enc_str;


type OptPath = Option<PathBuf>;

#[derive(Debug, Parser)]
#[command(name="tub")]
#[command(about="The most kickass DVCS of all?")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, action=ArgAction::SetTrue)]
    #[arg(help="Write buttloads of debuging stuffs to stderr")]
    verbose: bool,

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
        #[arg(help="Source directory (defaults to current CWD)")]
        source: Option<PathBuf>,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory")]
        tub: Option<PathBuf>,
    },
}


fn dir_or_cwd(target: OptPath) -> io::Result<PathBuf>
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

fn get_tub(target: OptPath) -> io::Result<Store>
{
    let target = dir_or_cwd(target)?;
    if let Ok(mut store) = find_store(&target) {
        store.reindex(false);
        Ok(store)
    }
    else {
        eprintln!("Could not find repository in {:?}", &target);
        exit(42);
    }
}


fn cmd_init(target: OptPath) -> io::Result<()>
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

fn cmd_import(source: OptPath, tub: OptPath) -> io::Result<()>
{
    let source = dir_or_cwd(source)?;
    let mut tub = get_tub(tub)?;
    let files = Scanner::scan_dir(&source)?;
    let mut buf: Vec<u8> = Vec::with_capacity(32 * 1024);
    for src in files.iter() {
        let mut fp = src.open()?;
        fp.read_to_end(&mut buf)?;
        let (id, new) = tub.add_object(&buf);
        println!("{} {:?} {:?}", &db32enc_str(&id), new, src.0);
        buf.clear();
    }
    Ok(())
}


pub fn run() -> io::Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::Init {target} => {
            cmd_init(target)
        }
        Commands::Import {source, tub} => {
            cmd_import(source, tub)
        }
    }
}
