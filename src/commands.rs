use std::path::{Path, PathBuf};
use std::env;
use std::io;
use std::fs;
use std::process::exit;
use std::time::Instant;

use clap::{Parser, Subcommand};

use crate::base::*;


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

    #[command(about = "😎 Create a new Tub repository 🛁")]
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

    #[command(about = "🔗 Verify all objects and blockchains 💵")]
    Check {},

    #[command(about = "🚀 Compare 🛁 hashing performance with git hash-object! 😜")]
    Hash {
        #[arg(help="Path of input file")]
        path: PathBuf,
    },
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
            not_yet()
        }
        Commands::Mov {tub, path} => {
            not_yet()
        }
        Commands::Rem {tub, path} => {
            not_yet()
        }
        Commands::Ignore {} => {
            not_yet()
        }
        Commands::Dif {} => {
            not_yet()
        }
        Commands::Status {tub} => {
            not_yet()
        }
        Commands::Commit {source, tub} => {
            cmd_commit(source, tub)
        }
        Commands::Revert {hash, dst, tub} => {
            not_yet()
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

fn get_tub(target: OptPath) -> io::Result<()>
{
    eprintln!("🛁❗ Could not find Tub in {:?}", &target);
    exit(42);
}


fn get_reindexed_tub(target: OptPath) -> io::Result<()> {
    Ok(())
}


fn not_yet() -> io::Result<()>
{
    eprintln!("🛁❗ Yo dawg, this command hasn't been implemented yet! 🤪");
    Ok(())
}


fn cmd_init(target: OptPath) -> io::Result<()>
{
    if false {
        eprintln!("🛁❗ Tub already exists: {:?}", "fixme");
        exit(42);
    }
    eprintln!("🛁 Created new Tub repository: {:?}", "fixme");
    eprintln!("🛁 Excellent first step, now reward yourself with two cookies! 🍪🍪");
    Ok(())
}


fn cmd_commit(source: OptPath, tub: OptPath) -> io::Result<()>
{
    eprintln!("🛁 Writing commit...");
    //println!("{}", db32enc(&root));
    eprintln!("🛁 Wow, great job on that one! 💋");
    Ok(())
}

fn cmd_hash(path: &Path) -> io::Result<()>
{
    let pb = path.canonicalize()?;
    let size = fs::metadata(&pb)?.len();
    let file = fs::File::open(&pb)?;
    eprintln!("🛁 Computing TubHash, this wont take long...");
    let start = Instant::now();
    let elapsed = start.elapsed().as_secs_f64();
    let rate = (size as f64 / elapsed) as u64;
    eprintln!("🛁 Hashed {} bytes in {}s, {} bytes/s", size, elapsed, rate);
    eprintln!("🛁 Holy fuck balls Blake3 is fast! 🚀");
    eprintln!("🛁 Run `time git hash-object` on the same file to compare 😲");
    eprintln!("🛁 The Blake3 reference implementation is even written in Rust!");
    eprintln!("🛁 Tub 💖 Rust, Tub 💖 Blake3");
    Ok(())
}

