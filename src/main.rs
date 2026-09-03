mod ai;
mod cli;
mod commands;
mod config;
mod tui;
mod vcs;

use crate::cli::{Cli, Command};
use crate::commands::{branch, commit, merge, pr, setup, squash, sync, update};
use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.version {
        println!("ggx {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    let result = tui::session(|| match cli.command {
        Some(Command::Setup) => setup::run(),
        Some(command) => {
            let provider = config::load()?;
            if !matches!(&command, Command::Update) {
                update::start_automatic();
            }

            match command {
                Command::Setup => unreachable!("setup is handled before configuration loading"),
                Command::Branch { prompt } => branch::run(provider, prompt),
                Command::Commit => commit::run(provider),
                Command::Pr {
                    draft,
                    closes,
                    base,
                } => pr::run(provider, draft, closes, base),
                Command::Sync => sync::run(),
                Command::Update => update::run(),
                Command::Merge { keep_branch, admin } => merge::run(keep_branch, admin),
                Command::Squash { keep_branch, admin } => squash::run(keep_branch, admin),
            }
        }
        None => unreachable!("clap requires a subcommand unless --version is set"),
    });

    if let Err(error) = result {
        tui::error(&error);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
