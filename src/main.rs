mod ai;
mod cli;
mod commands;
mod tui;
mod update;
mod vcs;

use crate::cli::{Cli, Command};
use crate::commands::{branch, commit, merge, pr, squash, sync};
use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = tui::session(|| {
        update::brew();

        match cli.command {
            Command::Branch { prompt } => branch::run(prompt),
            Command::Commit => commit::run(),
            Command::Pr { draft, closes } => pr::run(draft, closes),
            Command::Sync => sync::run(),
            Command::Merge {
                target,
                keep_branch,
                admin,
            } => merge::run(target, keep_branch, admin),
            Command::Squash { keep_branch, admin } => squash::run(keep_branch, admin),
        }
    });

    if let Err(error) = result {
        tui::error(&error);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
