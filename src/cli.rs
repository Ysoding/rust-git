use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};

use crate::{cat_file, hash_object, repo_create, repo_find};

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
    Checkout,
    Commit,
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
    Log,
    LsFiles,
    LsTree,
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
        Commands::Checkout => todo!(),
        Commands::Commit => todo!(),
        Commands::Log => todo!(),
        Commands::LsFiles => todo!(),
        Commands::LsTree => todo!(),
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
            let repo = repo_find(Path::new("."), true).unwrap().unwrap();
            cat_file(&repo, &object, Some(&object_type.as_bytes())).unwrap();
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
    }
}
