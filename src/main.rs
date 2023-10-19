use std::path::PathBuf;

use clap::{Parser, Subcommand};
use anyhow::Result;
use presquile::apply;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Audition CVS Markers file
    audition_cvs: PathBuf,

    /// Mp3 file
    mp3_file: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Write chapter to mp3 id3V2 tags from Adobe Audition CSV file
    Apply
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Apply => apply(cli.audition_cvs, cli.mp3_file),
    }
}