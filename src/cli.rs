use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::{cat_file, checkout, hash_object, log, ls_tree, repo_create};

#[derive(Parser)]
#[command(name="rit",version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, ValueEnum)]
enum ObjectType {
    Blob,
    Commit,
    Tag,
    Tree,
}

impl ObjectType {
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            ObjectType::Blob => b"blob",
            ObjectType::Commit => b"commit",
            ObjectType::Tag => b"tag",
            ObjectType::Tree => b"tree",
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    Add,
    /// Provide content of repository objects
    CatFile {
        #[arg(value_name = "type", help = "Specify the type", value_enum)]
        object_type: ObjectType,
        /// The object to display
        #[arg(value_name = "object")]
        object: String,
    },
    CheckIgnore,
    Commit,
    /// Checkout a commit inside of a directory.
    Checkout {
        /// The commit or tree to checkout.
        commit: String,
        /// The EMPTY directory to checkout on.
        path: PathBuf,
    },
    /// Compute object ID and optionally creates a blob from a file
    HashObject {
        #[arg(
            short='t',
            value_name = "type",
            help = "Specify the type",
            value_enum,
            default_value_t=ObjectType::Blob,
        )]
        object_type: ObjectType,

        /// Actually write the object into the database
        #[arg(short = 'w', value_name = "write", default_value_t = false)]
        write: bool,

        /// Read object from path
        #[arg(value_name = "path")]
        path: PathBuf,
    },
    /// Initialize a new, empty repository.
    Init {
        /// Where to create the repository.
        path: PathBuf,
    },
    /// Display history of a given commit.
    Log {
        /// Commit to start at.
        #[arg(default_value = "HEAD")]
        commit: String,
    },
    LsFiles,
    /// Pretty-print a tree object.
    LsTree {
        /// Recurse into sub-trees
        #[arg(short)]
        recursive: bool,
        /// A tree-ish object.
        tree: String,
    },
    RevParse,
    Rm,
    ShowRef,
    Status,
    Tag,
}

pub fn start() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add => todo!(),
        Commands::CheckIgnore => todo!(),
        Commands::Commit => todo!(),
        Commands::LsFiles => todo!(),
        Commands::RevParse => todo!(),
        Commands::Rm => todo!(),
        Commands::ShowRef => todo!(),
        Commands::Status => todo!(),
        Commands::Tag => todo!(),
        Commands::Init { path } => {
            repo_create(path).unwrap();
        }
        Commands::CatFile {
            object_type,
            object,
        } => {
            cat_file(&object, Some(&object_type.as_bytes())).unwrap();
        }
        Commands::HashObject {
            object_type,
            write,
            path,
        } => {
            println!(
                "{}",
                hash_object(&path, object_type.as_bytes(), write,).unwrap()
            );
        }
        Commands::Log { commit } => {
            log(&commit).unwrap();
        }
        Commands::LsTree { recursive, tree } => {
            ls_tree(&tree, recursive).unwrap();
        }
        Commands::Checkout { commit, path } => {
            checkout(&commit, &path).unwrap();
        }
    }
}
