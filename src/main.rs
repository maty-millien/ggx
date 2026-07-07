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

    if cli.version {
        println!("ggx {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    let result = tui::session(|| {
        update::brew();

        match cli.command {
            Some(Command::Branch { prompt }) => branch::run(prompt),
            Some(Command::Commit) => commit::run(),
            Some(Command::Pr { draft, closes }) => pr::run(draft, closes),
            Some(Command::Sync) => sync::run(),
            Some(Command::Merge {
                target,
                keep_branch,
                admin,
            }) => merge::run(target, keep_branch, admin),
            Some(Command::Squash { keep_branch, admin }) => squash::run(keep_branch, admin),
            None => unreachable!("clap requires a subcommand unless --version is set"),
        }
    });

    if let Err(error) = result {
        tui::error(&error);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
