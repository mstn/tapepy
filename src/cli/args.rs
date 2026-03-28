use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about = "Tapepy CLI")]
pub struct Cli {
    /// Root directory for tapepy runs and temporary files.
    #[arg(long)]
    pub home_folder: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
pub enum Command {
    /// Compile a source file into a tape hypergraph.
    Compile(CompileArgs),
}

#[derive(Parser)]
pub struct CompileArgs {
    /// Path to the source file.
    pub filepath: PathBuf,
    /// Source language (default: python).
    #[arg(long, default_value = "python")]
    pub language: String,
    /// Skip type solving and emit the raw tape hypergraph.
    #[arg(long)]
    pub raw_tape: bool,
}
