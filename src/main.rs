mod ai;
mod cli;
mod commands;
mod gh;
mod git;
mod tui;
mod update;

use crate::cli::{Cli, Command};
use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    update::brew_update();

    let result = match cli.command {
        Command::Branch { prompt } => commands::branch::run(prompt),
        Command::Commit => commands::commit::run(),
        Command::Pr {
            draft,
            base,
            closes,
        } => commands::pr::run(draft, base, closes),
        Command::Merge {
            target,
            keep_branch,
            admin,
        } => commands::merge::run(target, keep_branch, admin),
        Command::Squash { keep_branch, admin } => commands::squash::run(keep_branch, admin),
    };

    if let Err(error) = result {
        tui::error(&error);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
