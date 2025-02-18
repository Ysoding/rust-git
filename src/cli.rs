use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use crate::repo_create;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add,
    CatFile,
    CheckIgnore,
    Checkout,
    Commit,
    HashObject,
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
        Commands::CatFile => todo!(),
        Commands::CheckIgnore => todo!(),
        Commands::Checkout => todo!(),
        Commands::Commit => todo!(),
        Commands::HashObject => todo!(),
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
    }
}
