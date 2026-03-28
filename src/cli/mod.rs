pub(crate) mod args;
pub(crate) mod home;
pub(crate) mod output;

pub mod commands;

use std::error::Error;

use clap::Parser;

use self::args::{Cli, Command};
use self::home::HomeFolderLayout;

pub fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let home = HomeFolderLayout::new(cli.home_folder);
    match cli.command {
        Command::Compile(args) => commands::compile::run(&home, &args),
    }
}
