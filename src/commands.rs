use std::path::{Path, PathBuf};
use std::env;
use std::io;
use std::fs;
use std::process::exit;

use clap::{ArgAction, Parser, Subcommand};

use crate::base::*;
use crate::store::{find_store, init_tree, Store};
use crate::importer::Scanner;
use crate::dbase32::{db32enc_str, db32dec_into};
use crate::leaf_io::hash_object;


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

    #[command(about = "Calculate the Tub-Hash of a file")]
    HashObject {
        #[arg(help="Path of input file")]
        path: PathBuf,
    },

    #[command(about = "Print hash of each object specified Tub.")]
    ListObjects {
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "Delete object from object store")]
    DelObject {
        #[arg(help="Source directory (defaults to current CWD)")]
        hash: String,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory")]
        tub: Option<PathBuf>,
    },

    #[command(about = "Cat object data to file or stdout")]
    CatFile {
        #[arg(help="Source directory (defaults to current CWD)")]
        hash: String,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory")]
        tub: Option<PathBuf>,

        #[arg(help="File name to write data to")]
        dst: Option<PathBuf>,
    },
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
        Commands::HashObject {path} => {
            cmd_hash(&path)
        }
        Commands::ListObjects {tub} => {
            cmd_list_objects(tub)
        }
        Commands::DelObject {tub, hash} => {
            cmd_obj_del(tub, hash)
        }
        Commands::CatFile {hash, tub, dst} => {
            cmd_obj_cat(hash, tub, dst)
        }
    }
}


fn decode_hash(txt: &String) -> Option<TubHash>
{
    if txt.len() != 48 {
        eprintln!("Tub-Hash must be 48 characters, got {}: {:?}", txt.len(), txt);
        exit(42);
    }
    let mut bin = [0_u8; TUB_HASH_LEN];
    if db32dec_into(txt.as_bytes(), &mut bin) {
        Some(bin)
    }
    else {
        None
    }
    
}



fn dir_or_cwd(target: OptPath) -> io::Result<PathBuf>
{
    let pb = match target {
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
        store.reindex()?;
        println!("Using store {:?}", store.path());
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
    if let Ok(_store) = find_store(&target) {
        eprintln!("Tub already exists: {:?}", target);
        exit(42);
    }
    else if let Ok(store) = init_tree(&target) {
        eprintln!("Created Tub: {:?}", store.path());
    }
    Ok(())
}

fn cmd_import(source: OptPath, tub: OptPath) -> io::Result<()>
{
    let source = dir_or_cwd(source)?;
    let mut tub = get_tub(tub)?;
    let files = Scanner::scan_dir(&source)?;
    for src in files.iter() {
        let (root, new) = tub.import_file(src.open()?)?;
        println!("{} {} {:?}", root, new, src.path);
    }
    Ok(())
}

fn cmd_hash(path: &Path) -> io::Result<()>
{
    let pb = path.canonicalize()?;
    let file = fs::File::open(&pb)?;
    let tt = hash_object(file)?;
    println!("{}", tt);
    Ok(())
}

fn cmd_list_objects(tub: OptPath) -> io::Result<()>
{
    let tub = get_tub(tub)?;
    let mut keys = tub.keys();
    keys.sort();
    for hash in keys {
        println!("{}", db32enc_str(&hash));
    }
    eprintln!("{} objects in store", tub.len());
    Ok(())
}

fn cmd_obj_del(tub: OptPath, txt: String) -> io::Result<()>
{
    if txt.len() != 48 {
        eprintln!("Tub-Hash must be 48 characters, got {}: {:?}", txt.len(), txt);
        exit(42);
    }
    if let Some(hash) = decode_hash(&txt) {
        println!("good db32: {}", txt);
        let mut tub = get_tub(tub)?;
        tub.delete_object(&hash)?;
    }
    else {
        println!("Invalid Dbase32 encoding: {:?}", txt);
        exit(42);
    }
    Ok(())
}

fn cmd_obj_cat(txt: String, tub: OptPath, dst: OptPath) -> io::Result<()>
{
    if txt.len() != 48 {
        eprintln!("Tub-Hash must be 48 characters, got {}: {:?}", txt.len(), txt);
        exit(42);
    }
    if let Some(hash) = decode_hash(&txt) {
        let tub = get_tub(tub)?;
        let pb: PathBuf = match dst {
            Some(inner) => {
                inner
            }
            None => {
                PathBuf::from(txt)
            }
        };
        if let Some(mut obj) = tub.open(&hash)? {
            eprintln!("Writting to {:?}", &pb);
            let mut file = fs::File::options()
                            .create_new(true)
                            .append(true).open(&pb)?;
            obj.write_to_file(&mut file)?;
        }
    }
    Ok(())
}

