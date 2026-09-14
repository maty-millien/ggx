mod ai;
mod cli;
mod commands;
mod config;
mod tui;
mod vcs;

use crate::cli::{Cli, Command};
use crate::commands::{branch, commit, merge, pr, setup, squash, sync, update};
use clap::{CommandFactory, FromArgMatches};
use std::process::ExitCode;

fn main() -> ExitCode {
    let matches = Cli::command()
        .mut_subcommands(|command| {
            if command.get_name() == "setup" {
                command
            } else {
                command.arg(
                    clap::Arg::new("yes")
                        .short('y')
                        .long("yes")
                        .help("Automatically confirm actions without terminal input")
                        .action(clap::ArgAction::SetTrue),
                )
            }
        })
        .get_matches();
    let yes = matches
        .subcommand()
        .is_some_and(|(name, args)| name != "setup" && args.get_flag("yes"));
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    let interactive = !yes && !matches!(&cli.command, Some(Command::Setup { provider: Some(_) }));

    if cli.version {
        println!("ggx {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    let result = tui::session(interactive, || match cli.command {
        Some(Command::Setup { provider }) => setup::run(provider),
        Some(command) => {
            let provider = config::load()?;
            if !matches!(&command, Command::Update) {
                update::start_automatic();
            }

            match command {
                Command::Setup { .. } => {
                    unreachable!("setup is handled before configuration loading")
                }
                Command::Branch { prompt } => branch::run(provider, prompt, yes),
                Command::Commit => commit::run(provider, yes),
                Command::Pr {
                    draft,
                    closes,
                    base,
                } => pr::run(provider, draft, closes, base, yes),
                Command::Sync => sync::run(yes),
                Command::Update => update::run(),
                Command::Merge { keep_branch, admin } => merge::run(keep_branch, admin, yes),
                Command::Squash { keep_branch, admin } => squash::run(keep_branch, admin, yes),
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
