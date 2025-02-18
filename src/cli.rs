use clap::{Parser, Subcommand};

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
    Init,
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
        Commands::Init => todo!(),
        Commands::Log => todo!(),
        Commands::LsFiles => todo!(),
        Commands::LsTree => todo!(),
        Commands::RevParse => todo!(),
        Commands::Rm => todo!(),
        Commands::ShowRef => todo!(),
        Commands::Status => todo!(),
        Commands::Tag => todo!(),
    }
}
