mod ai;
mod cli;
mod commands;
mod git;
mod ui;

use crate::cli::{Cli, Command};
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Commit => commands::commit::run()?,
    }

    Ok(())
}
