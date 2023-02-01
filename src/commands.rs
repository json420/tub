//! CLI commands for WIP version control tool `tub`.

use std::path::{Path, PathBuf};
use std::env;
use std::io;
use std::fs;
use std::process::exit;
use std::time::Instant;
use clap::{Parser, Subcommand};
use sodiumoxide;
use crate::chaos::{DefaultObject, DefaultName};
use crate::tub::{find_dotdir, DefaultTub};
use crate::dvcs::{DefaultTree, DefaultCommit};
use crate::inception::hash_file;

type OptPath = Option<PathBuf>;

#[derive(Debug, Parser)]
#[command(name="tub")]
#[command(about="🛁 Tub: Relaxing version control for everyone! 🌎 💖 🦀 🦓")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}


#[derive(Debug, Subcommand)]
enum Commands {

    #[command(about = "😎 Create a new Tub repository 🛁")]
    Init {
        #[arg(help = "Target directory (defaults to CWD)")]
        target: Option<PathBuf>,
    },

    #[command(about = "👷 Fork 🥄 history into a new indpendent branch 🪛")]
    Branch {},

    #[command(about = "🔴 Add paths to tracking list")]
    Add {
        #[arg(help="Paths to add to tracking list")]
        paths: Vec<PathBuf>,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "🟡 Rename a tracked path")]
    Mov {
        #[arg(help="Path to rename")]
        src: String,

        #[arg(help="New name")]
        dst: String,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "🟢 Remove paths from tracking list")]
    Rem {
        #[arg(help="Path to remove")]
        paths: Vec<String>,

        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "🚫 Add paths to ignore list")]
    Ignore {
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,

        #[arg(help="path names to ignore (or unignore)")]
        paths: Vec<String>,

        #[arg(short, long, help="Remove paths from ignore list")]
        remove: bool,
    },

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
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory", hide=true)]
        tub: Option<PathBuf>,

        #[arg(short, long, value_name="MESSAGE")]
        #[arg(help="Short description of this commit")]
        msg: Option<String>,
    },

    #[command(about = "🧬 Bring changes from one branch into another 😍")]
    Merge {},

    #[command(about = "🚽 Undo 💩 changes in working tree")]
    Revert {
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,

        #[arg(help="Dbase32-encoded hash")]
        hash: String,
    },

    #[command(about = "📜 View commit history")]
    Log {
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory (defaults to CWD)")]
        tub: Option<PathBuf>,
    },

    #[command(about = "🔗 Verify all objects and blockchains 💵")]
    Check {
        #[arg(short, long, value_name="DIR")]
        #[arg(help="Path of Tub control directory")]
        tub: Option<PathBuf>,
    },

    #[command(about = "🚀 Compare 🛁 hashing performance with git hash-object! 😜")]
    Hash {
        #[arg(help="Path of input file")]
        path: PathBuf,
    },
}


pub fn run() -> io::Result<()> {
    sodiumoxide::init().unwrap();
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
        Commands::Add {tub, paths} => {
            cmd_add(tub, paths)
        }
        Commands::Mov {tub, src, dst} => {
            not_yet()
        }
        Commands::Rem {tub, paths} => {
            not_yet()
        }
        Commands::Ignore {tub, paths, remove} => {
            cmd_ignore(tub, paths, remove)
        }
        Commands::Dif {} => {
            not_yet()
        }
        Commands::Status {tub} => {
            cmd_status(tub)
        }
        Commands::Commit {tub, msg} => {
            cmd_commit(tub, msg)
        }
        Commands::Revert {tub, hash} => {
            cmd_revert(tub, hash)
        }
        Commands::Log {tub} => {
            cmd_log(tub)
        }
        Commands::Check {tub} => {
            cmd_check(tub)
        }
        Commands::Hash {path} => {
            cmd_hash(&path)
        }
    }
}


macro_rules! other_err {
    ($msg:literal) => {
        Err(io::Error::new(io::ErrorKind::Other, $msg))
    }
}


fn dir_or_cwd(target: OptPath) -> io::Result<PathBuf>
{
    let pb = match target {
        Some(dir) => dir,
        None => env::current_dir()?,
    };
    if pb.is_dir() {
        pb.canonicalize()
    }
    else {
        eprintln!("🛁❗ Not a directory: {:?}", pb);
        other_err!("Not a dir")
    }
}


fn get_tub(target: &Path) -> io::Result<DefaultTub>
{
    if let Some(dotdir) = find_dotdir(&target) {
        let mut tub = DefaultTub::open(dotdir)?;
        tub.reindex()?;
        Ok(tub)
    }
    else {
        other_err!("Could not find Tub")
    }
}


fn get_tub_exit(target: &Path) -> io::Result<DefaultTub>
{
    if let Ok(tub) = get_tub(&target) {
        Ok(tub)
    }
    else {
        eprintln!("🛁❗ Could not find Tub in {:?}", &target);
        exit(42);
    }
}


fn not_yet() -> io::Result<()>
{
    eprintln!("🛁❗ Yo dawg, this command hasn't been implemented yet! 🤪");
    Ok(())
}


fn cmd_init(target: OptPath) -> io::Result<()>
{
    let target = dir_or_cwd(target)?;
    if let Ok(tub) = get_tub(&target) {
        eprintln!("🛁❗ Tub already exists: {:?}", tub.dotdir());
        exit(42);
    }
    else {
        let tub = DefaultTub::create(&target)?;
        tub.create_branch()?;
        eprintln!("🛁 Created new Tub repository: {:?}", tub.dotdir());
        eprintln!("🛁 Excellent first step, now reward yourself with two cookies! 🍪🍪");
        Ok(())
    }
}


fn cmd_add(tub: OptPath, paths: Vec<PathBuf>) -> io::Result<()> {
    let tub = get_tub_exit(&dir_or_cwd(tub)?)?;
    for p in paths {
        if ! p.exists() {
            eprintln!("🛁❗Path does not exists: {:?}", p);
            exit(42);
        }
        println!("{:?}", p);
    }
    Ok(())
}


fn cmd_commit(tub: OptPath, msg: Option<String>) -> io::Result<()>
{
    let tub = get_tub_exit(&dir_or_cwd(tub)?)?;
    let mut source = tub.treedir();
    let mut chain = tub.open_branch()?;
    if ! tub.load_branch_seckey(&mut chain)? {
        eprintln!("🛁❗ Cannot find key for {}", chain.header.hash());
        exit(42);
    }
    let store = tub.into_store();
    let mut obj = store.new_object();
    let mut scanner = DefaultTree::new(store, &source);
    scanner.load_ignore()?;
    scanner.enable_import();
    eprintln!("🛁 Writing commit...");
    if let Some(root) = scanner.scan_tree()? {
        let msg = if let Some(msg) = msg {msg} else {String::from("")};
        let commit = DefaultCommit::new(root, msg);
        obj.clear();
        commit.serialize(obj.as_mut_vec());
        obj.finalize_with_kind(69);
        let mut store = scanner.into_store();
        store.save(&obj)?;
        chain.sign_next(&obj.hash())?;
        println!("{}", &obj.hash());
    }
    eprintln!("🛁 Wow, great job on that one! 💋");
    Ok(())
}


fn cmd_status(tub: OptPath) -> io::Result<()>
{
    let tub = get_tub_exit(&dir_or_cwd(tub)?)?;
    let source = tub.treedir();
    let mut chain = tub.open_branch()?;
    if chain.load_last_block()? {
        let mut store = tub.into_store();
        let mut obj = store.new_object();

        if store.load(&chain.block.payload(), &mut obj)? {
            let commit = DefaultCommit::deserialize(obj.as_data());
            eprintln!(" block: {}", chain.block.hash());
            eprintln!("commit: {}", chain.block.payload());
            eprintln!("   old: {}", commit.tree);

            let mut scanner = DefaultTree::new(store, &source);
            scanner.load_ignore()?;
            let a = scanner.flatten_tree(&commit.tree)?;
            let root = scanner.scan_tree()?.unwrap();
            eprintln!("   new: {}", root);
            let mut status = scanner.compare_with_flatmap(&a);
            status.sort();
            if status.removed.len() > 0 {
                println!("Removed:");
                for relname in status.removed.iter() {
                    println!("  {}", relname);
                }
            }
            if status.changed.len() > 0 {
                println!("Changed:");
                for relname in status.changed.iter() {
                    println!("  {}", relname);
                }
            }
            if status.unknown.len() > 0 {
                println!("Unknown:");
                for relname in status.unknown.iter() {
                    println!("  {}", relname);
                }
            }
        }
    }
    else {
        eprintln!("🛁 Status: it's complicated! 🤣");
        eprintln!("🛁 Status: empty project, get to work, yo!");
    }
    /*
    let mut scanner = DefaultTree::new(tub.into_store(), &source);
    eprintln!("🛁 Scanning tree state, wont take long...");
    if let Some(root) = scanner.scan_tree()? {
        println!("{}", root);
    }
    */
    Ok(())
}


// FIXME: Use this - https://docs.rs/glob/latest/glob/struct.Pattern.html
fn cmd_ignore(tub: OptPath, paths: Vec<String>, remove: bool) -> io::Result<()>
{
    let tub = get_tub_exit(&dir_or_cwd(tub)?)?;
    let mut source = tub.treedir();
    let store = tub.into_store();
    let mut obj = store.new_object();
    let mut tree = DefaultTree::new(store, &source);

    tree.load_ignore()?;
    if remove {
        for relpath in paths.iter() {
            tree.unignore(relpath);
        }
    }
    else {
        for relpath in paths.iter() {
            tree.ignore(relpath.to_owned());
        }
    }
    if paths.len() > 0 {
        tree.save_ignore()?;
    }

    eprintln!("🚫 Ignored paths:");
    for relpath in tree.sorted_ignore_vec() {
        println!("{}", relpath);
    }
    Ok(())
}


fn cmd_revert(tub: OptPath, txt: String) -> io::Result<()> {
    let hash = DefaultName::from_str(&txt);
    let tub = get_tub_exit(&dir_or_cwd(tub)?)?;
    let dst = tub.treedir();
    let store = tub.into_store();
    let mut scanner = DefaultTree::new(store, &dst);
    scanner.restore_tree(&hash)?;
    Ok(())
}

fn cmd_log(tub: OptPath) -> io::Result<()>
{
    let tub = get_tub_exit(&dir_or_cwd(tub)?)?;
    if let Ok(mut chain) = tub.open_branch() {
        let mut store = tub.into_store();
        let mut obj = store.new_object();
        chain.seek_to_beyond();
        while chain.load_previous()? {
            println!(" block: {} {}", chain.block.hash(), chain.block.index());
            println!("commit: {}", chain.block.payload());
            if store.load(&chain.block.payload(), &mut obj)? {
                let commit = DefaultCommit::deserialize(obj.as_data());
                println!("  tree: {}", commit.tree);
                println!("📜 {}", commit.msg);
            }
            println!("");
        }
    }
    else {
        eprintln!("🛁 No commits yet, get to work! 💵");
    }
    Ok(())
}

fn cmd_check(tub: OptPath) -> io::Result<()>
{
    let mut tub = get_tub_exit(&dir_or_cwd(tub)?)?;
    let start = Instant::now();
    eprintln!("🛁 Verifying {} objects...", tub.store.len());
    tub.check()?;
    let elapsed = start.elapsed().as_secs_f64();
    let size = tub.store.size();
    let rate = (size as f64 / elapsed) as u64;
    eprintln!("🛁 Verified {} bytes in {}s, {} bytes/s", size, elapsed, rate);
    Ok(())
}


fn cmd_hash(path: &Path) -> io::Result<()>
{
    let start = Instant::now();
    let pb = path.canonicalize()?;
    let size = fs::metadata(&pb)?.len();
    let file = fs::File::open(&pb)?;
    let mut obj = DefaultObject::new();
    eprintln!("🛁 Computing TubHash, this wont take long...");
    println!("{}", hash_file(&mut obj, file, size)?);
    let elapsed = start.elapsed().as_secs_f64();
    let rate = (size as f64 / elapsed) as u64;
    eprintln!("🛁 Hashed {} bytes in {}s, {} bytes/s", size, elapsed, rate);
    eprintln!("🛁 Holy fuck balls Blake3 is fast! 🚀");
    eprintln!("🛁 Run `time git hash-object` on the same file to compare 😲");
    eprintln!("🛁 The Blake3 reference implementation is even written in Rust!");
    eprintln!("🛁 Tub 💖 Rust, Tub 💖 Blake3");
    Ok(())
}

