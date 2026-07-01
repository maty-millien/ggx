mod ai;
mod cli;
mod commands;
mod git;
mod ui;

use crate::cli::{Cli, Command};
use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Commit => commands::commit::run(),
    };

    if let Err(error) = result {
        ui::error(&error);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
