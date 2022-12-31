use std::path::{Path, PathBuf};
use std::env;
use std::io;
use std::fs;
use std::process::exit;

use clap::{ArgAction, Parser, Subcommand};

use crate::base::*;
use crate::store::{find_store, init_tree, Store};
use crate::importer::Scanner;
use crate::dbase32::{db32enc, db32dec_into};
use crate::leaf_io::hash_file;
use crate::dvcs;


type OptPath = Option<PathBuf>;

#[derive(Debug, Parser)]
#[command(name="tub")]
#[command(about="Tub 💖 Rust")]
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

    #[command(about = "Recursively commit directory")]
    CommitTree {
        #[arg(help="Tree directory (defaults to current CWD)")]
        source: Option<PathBuf>,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory")]
        tub: Option<PathBuf>,
    },

    #[command(about = "Restore tree from root tree hash")]
    RestoreTree {
        #[arg(help="Dbase32-encoded hash")]
        hash: String,

        #[arg(help="Target directory (defaults to current CWD)")]
        dst: Option<PathBuf>,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory")]
        tub: Option<PathBuf>,
    },

    #[command(about = "Calculate the Tub-Hash of a file")]
    HashObject {
        #[arg(help="Path of input file")]
        path: PathBuf,
    },

    #[command(about = "Print hash of each object specified Tub")]
    ListObjects {
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "Print summary about objets in Tub")]
    Stats {
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "List tracked paths")]
    Ls {
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "Add path to tracking list")]
    Add {
        #[arg(help="Source directory (defaults to current CWD)")]
        path: String,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "Cat object data to file or stdout")]
    CatFile {
        #[arg(help="Dbase32-encoded hash")]
        hash: String,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory")]
        tub: Option<PathBuf>,

        #[arg(help="File name to write data to")]
        dst: Option<PathBuf>,
    },

    #[command(about = "Repack and remove tombstones")]
    Repack {
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
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
        Commands::CommitTree {source, tub} => {
            cmd_commit_tree(source, tub)
        }
        Commands::RestoreTree {hash, dst, tub} => {
            cmd_restore_tree(hash, dst, tub)
        }
        Commands::HashObject {path} => {
            cmd_hash(&path)
        }
        Commands::ListObjects {tub} => {
            cmd_list_objects(tub)
        }
        Commands::Stats {tub} => {
            cmd_stats(tub)
        }
        Commands::Ls {tub} => {
            cmd_ls(tub)
        }
        Commands::Add {tub, path} => {
            cmd_add(tub, path)
        }
        Commands::CatFile {hash, tub, dst} => {
            cmd_obj_cat(hash, tub, dst)
        }
        Commands::Repack {tub} => {
            cmd_repack(tub)
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
        //store.reindex()?;
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


fn get_newmark(new: bool) -> String {
    let m = if new {" "} else {"!"};
    String::from(m)
}

fn get_largemark(large: bool) -> String {
    let m = if large {"L"} else {" "};
    String::from(m)
}

fn cmd_import(source: OptPath, tub: OptPath) -> io::Result<()>
{
    let source = dir_or_cwd(source)?;
    let mut tub = get_tub(tub)?;
    let files = Scanner::scan_dir(&source)?;

    let mut new_cnt = 0_u64;
    let mut dup_cnt = 0_u64;
    for src in files.iter() {
        let (root, new) = tub.import_file(src.open()?, src.size)?;
        println!("{} {}{} {:?}",
            db32enc(&root),
            get_largemark(src.is_large()),
            get_newmark(new),
            src.path
        );
        if new {
            new_cnt += 1;
        } else {
            dup_cnt += 1;
        }
    }
    eprintln!("Imported {} new files and {} duplicates", new_cnt, dup_cnt);
    Ok(())
}

fn cmd_commit_tree(source: OptPath, tub: OptPath) -> io::Result<()>
{
    let source = dir_or_cwd(source)?;
    let mut tub = get_tub(tub)?;
    let root = dvcs::commit_tree(&mut tub, &source)?;
    println!("{}", db32enc(&root));
    let commit = dvcs::Commit::new(root, String::from("test commit"));
    tub.add_commit(&commit.serialize())?;
    Ok(())
}

fn cmd_restore_tree(txt: String, dst: OptPath, tub: OptPath) -> io::Result<()>
{
    if let Some(hash) = decode_hash(&txt) {
        let dst = dir_or_cwd(dst)?;
        let mut tub = get_tub(tub)?;
        dvcs::restore_tree(&mut tub, &hash, &dst)?;
    }
    Ok(())
}

fn cmd_hash(path: &Path) -> io::Result<()>
{
    let pb = path.canonicalize()?;
    let size = fs::metadata(&pb)?.len();
    let file = fs::File::open(&pb)?;
    let tt = hash_file(file, size)?;
    println!("{}", tt);
    Ok(())
}

fn cmd_list_objects(tub: OptPath) -> io::Result<()>
{
    let tub = get_tub(tub)?;
    let mut keys = tub.keys();
    keys.sort();
    for hash in keys {
        println!("{}", db32enc(&hash));
    }
    eprintln!("{} objects in store", tub.len());
    Ok(())
}

fn cmd_stats(tub: OptPath) -> io::Result<()>
{
    let tub = get_tub(tub)?;
    let stats = tub.stats();
    println!("Tub contains {} objects ({} bytes)", tub.len(), stats.total);
    println!("Objects by size:");
    println!("  {} Large ({} bytes)", stats.large.count, stats.large.total);
    println!("  {} Small ({} bytes)", stats.small.count, stats.small.total);
    println!("Objects by type:");
    println!("  {} Data ({} bytes)", stats.data.count, stats.data.total);
    println!("  {} Tree ({} bytes)", stats.tree.count, stats.tree.total);
    println!("  {} Block ({} bytes)", stats.block.count, stats.block.total);
    println!("  {} Commit ({} bytes)", stats.commit.count, stats.commit.total);
    Ok(())
}

fn cmd_add(tub: OptPath, path: String) -> io::Result<()>
{
    let tub = get_tub(tub)?;
    let wt = dvcs::WorkingTree::new(tub);
    let mut tl = wt.load_tracking_list()?;
    tl.add(path);
    wt.save_tracking_list(tl)?;
    println!("yo from cmd_add");
    Ok(())
}

fn cmd_ls(tub: OptPath) -> io::Result<()>
{
    let tub = get_tub(tub)?;
    let wt = dvcs::WorkingTree::new(tub);
    let tl = wt.load_tracking_list()?;
    for path in tl.as_sorted_vec() {
        println!("{}", path);
    }
    eprintln!("\t{} item(s) in tracking list", tl.len());
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

fn cmd_repack(tub: OptPath) -> io::Result<()>
{
    let mut tub = get_tub(tub)?;
    tub.repack()?;
    eprintln!("{} objects in store", tub.len());
    Ok(())
}

