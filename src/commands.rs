use std::path::{Path, PathBuf};
use std::env;
use std::io;
use std::fs;
use std::process::exit;
use std::time::Instant;

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
#[command(about="🛁 Tub: Relaxing version control for everyone! 🌎 💖 🦓")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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

    #[command(about = "😎 Create a new Tub 🛁 repository")]
    Init {
        #[arg(help = "Target directory (defaults to CWD)")]
        target: Option<PathBuf>,
    },

    #[command(about = "👷 Fork 🥄 history into a new indpendent branch 🪛")]
    Branch {},

    #[command(about = "🔴 Add paths to tracking list")]
    Add {
        #[arg(help="Path to add")]
        path: String,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "🟡 Rename a tracked path")]
    Mov {
        #[arg(help="Path to rename")]
        path: String,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "🟢 Remove paths from tracking list")]
    Rem {
        #[arg(help="Path to remove")]
        path: String,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "🚫 Add paths to ignore list")]
    Ignore {},

    #[command(about = "🔎 Examine changes in working tree")]
    Dif {},

    #[command(about = "🤔 Sumarize changes in working tree")]
    Status {
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "💖 Take a snapshot 📸 of your work 🤓")]
    Commit {
        #[arg(help="Tree directory (defaults to current CWD)")]
        source: Option<PathBuf>,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory")]
        tub: Option<PathBuf>,
    },

    #[command(about = "🧬 Insert changes from one branch into another 😍")]
    Merge {},

    #[command(about = "🚽 Undo 💩 changes in working tree")]
    Revert {
        #[arg(help="Dbase32-encoded hash")]
        hash: String,

        #[arg(help="Target directory (defaults to current CWD)")]
        dst: Option<PathBuf>,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory")]
        tub: Option<PathBuf>,
    },

    #[command(about = "📜 View commit history")]
    Log {},

    #[command(about = "🔗 Verify all objects, blockchains, and metadata")]
    Check {},

    #[command(about = "🚀 Compare 🛁 hashing performance with git hash-object! 😜")]
    Hash {
        #[arg(help="Path of input file")]
        path: PathBuf,
    },

    /*
    #[command(about = "Recursively import files from directory")]
    Import {
        #[arg(help="Source directory (defaults to current CWD)")]
        source: Option<PathBuf>,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory")]
        tub: Option<PathBuf>,
    },

    #[command(about = "Print hash of each object specified Tub")]
    ListObjects {
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "Repack and remove tombstones")]
    Repack {
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },
    */
}


pub fn run() -> io::Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::Init {target} => {
            cmd_init(target)
        }
        Commands::Branch {} => {
            not_yet()
        }
        Commands::Merge {} => {
            not_yet()
        }
        Commands::Add {tub, path} => {
            cmd_add(tub, path)
        }
        Commands::Mov {tub, path} => {
            cmd_add(tub, path)
        }
        Commands::Rem {tub, path} => {
            cmd_rm(tub, path)
        }
        Commands::Ignore {} => {
            not_yet()
        }
        Commands::Dif {} => {
            not_yet()
        }
        Commands::Status {tub} => {
            cmd_ls(tub)
        }
        Commands::Commit {source, tub} => {
            cmd_commit_tree(source, tub)
        }
        Commands::Revert {hash, dst, tub} => {
            cmd_restore_tree(hash, dst, tub)
        }
        Commands::Log {} => {
            not_yet()
        }
        Commands::Check {} => {
            not_yet()
        }
        Commands::Hash {path} => {
            cmd_hash(&path)
        }
        /*
        Commands::Import {source, tub} => {
            cmd_import(source, tub)
        }
        Commands::ListObjects {tub} => {
            cmd_list_objects(tub)
        }
        Commands::Repack {tub} => {
            cmd_repack(tub)
        }
        */
    }
}


fn decode_hash(txt: &String) -> Option<TubHash>
{
    if txt.len() != 48 {
        eprintln!("🛁 Tub-Hash must be 48 characters, got {}: {:?}", txt.len(), txt);
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
        eprintln!("🛁❗ Not a directory: {:?}", pb);
        exit(42);
    }
    Ok(pb.canonicalize()?)
}

fn get_tub(target: OptPath) -> io::Result<Store>
{
    let target = dir_or_cwd(target)?;
    if let Ok(mut store) = find_store(&target) {
        Ok(store)
    }
    else {
        eprintln!("🛁❗ Could not find Tub in {:?}", &target);
        exit(42);
    }
}


fn get_reindexed_tub(target: OptPath) -> io::Result<Store> {
    let mut tub = get_tub(target)?;
    tub.reindex()?;
    Ok(tub)
}


fn cmd_init(target: OptPath) -> io::Result<()>
{
    let target = dir_or_cwd(target)?;
    if let Ok(store) = find_store(&target) {
        eprintln!("🛁❗ Tub already exists: {:?}", store.path());
        exit(42);
    }
    else if let Ok(store) = init_tree(&target) {
        eprintln!("🛁 Created new Tub repository: {:?}", store.path());
        eprintln!("🛁 Excellent first step, now reward yourself with two cookies! 🍪🍪");
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


fn not_yet() -> io::Result<()>
{
    eprintln!("🛁❗ Yo dawg, this command hasn't been implemented yet! 🤪");
    Ok(())
}

fn cmd_import(source: OptPath, tub: OptPath) -> io::Result<()>
{
    let source = dir_or_cwd(source)?;
    let mut tub = get_reindexed_tub(tub)?;
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
    eprintln!("🛁 Imported {} new files and {} duplicates", new_cnt, dup_cnt);
    Ok(())
}

fn cmd_commit_tree(source: OptPath, tub: OptPath) -> io::Result<()>
{
    let source = dir_or_cwd(source)?;
    let mut tub = get_tub(tub)?;
    eprintln!("🛁 Writing commit...");
    let root = dvcs::commit_tree(&mut tub, &source)?;
    let commit = dvcs::Commit::new(root, String::from("test commit"));
    tub.add_commit(&commit.serialize())?;
    println!("{}", db32enc(&root));
    eprintln!("🛁 Wow, great job on that one! 💋");
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
    eprintln!("🛁 Computing TubHash, this wont take long...");
    let start = Instant::now();
    let tt = hash_file(file, size)?;
    println!("{}", tt);
    let elapsed = start.elapsed().as_secs_f64();
    let rate = (size as f64 / elapsed) as u64;
    eprintln!("🛁 Hashed {} bytes in {}s, {} bytes/s", size, elapsed, rate);
    eprintln!("🛁 Holy fuck balls Blake3 is fast! 🚀");
    eprintln!("🛁 Run `time git hash-object` on the same file to compare 😲");
    eprintln!("🛁 The Blake3 reference implementation is even written in Rust!");
    eprintln!("🛁 Tub 💖 Rust, Tub 💖 Blake3");
    Ok(())
}


fn cmd_list_objects(tub: OptPath) -> io::Result<()>
{
    let tub = get_reindexed_tub(tub)?;
    let mut keys = tub.keys();
    keys.sort();
    for hash in keys {
        println!("{}", db32enc(&hash));
    }
    eprintln!("🛁 {} objects in Tub", tub.len());
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
    if tl.add(path.clone()) {
        eprintln!("🛁 Added '{}' to tracking list", path);
        eprintln!("🛁 This is getting exciting, let me grab my popcorn 🍿");
        wt.save_tracking_list(tl)?;
    }
    else {
        eprintln!("🛁❗ '{}' is already tracked", path);
    }
    Ok(())
}

fn cmd_rm(tub: OptPath, path: String) -> io::Result<()>
{
    let tub = get_tub(tub)?;
    let wt = dvcs::WorkingTree::new(tub);
    let mut tl = wt.load_tracking_list()?;
    if tl.remove(&path) {
        eprintln!("🛁 Removed '{}' from tracking list", path);
        wt.save_tracking_list(tl)?;
    }
    else {
        eprintln!("🛁❗ '{}' is not a tracked file", path);
    }
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
    eprintln!("🛁 {} item(s) in tracking list", tl.len());
    Ok(())
}

fn cmd_repack(tub: OptPath) -> io::Result<()>
{
    let mut tub = get_tub(tub)?;
    tub.repack()?;
    eprintln!("🛁 {} objects in store", tub.len());
    Ok(())
}

