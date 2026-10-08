mod ai;
mod commands;
mod config;
mod git;
mod github;
mod tui;

use ai::Provider;
use clap::Parser;
use commands::{branch, commit, merge, pr, setup, sync, update};
use std::ffi::OsStr;
use std::process::{Command, ExitCode};

#[derive(clap::Parser)]
#[command(version, disable_version_flag = true, arg_required_else_help = true)]
struct Cli {
    #[arg(short = 'v', long, action = clap::ArgAction::Version)]
    version: (),
    /// Automatically confirm actions without terminal input
    #[arg(short, long, global = true)]
    yes: bool,
    #[command(subcommand)]
    action: Action,
}

#[derive(clap::Subcommand)]
enum Action {
    Setup {
        #[arg(long)]
        provider: Option<Provider>,
    },
    Branch {
        prompt: Option<String>,
    },
    Commit,
    Pr {
        #[arg(long)]
        draft: bool,
        #[arg(long)]
        closes: Vec<String>,
        #[arg(long)]
        base: Option<String>,
    },
    Sync,
    Update,
    Merge {
        #[arg(long)]
        keep_branch: bool,
        #[arg(long)]
        admin: bool,
    },
    Squash {
        #[arg(long)]
        keep_branch: bool,
        #[arg(long)]
        admin: bool,
    },
}

fn main() -> ExitCode {
    let Cli { yes, action, .. } = Cli::parse();
    // Setup ignores -y: it asks unless --provider is given.
    let interactive = match &action {
        Action::Setup { provider } => provider.is_none(),
        _ => !yes,
    };

    if let Err(error) = tui::session(interactive, || dispatch(action, yes)) {
        tui::error(&error);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn dispatch(action: Action, yes: bool) -> anyhow::Result<()> {
    let configured = || {
        let provider = config::load()?;
        update::start_automatic();
        anyhow::Ok(provider)
    };

    match action {
        Action::Setup { provider } => setup::run(provider),
        Action::Update => config::load().and_then(|_| update::run()),
        Action::Branch { prompt } => branch::run(configured()?, prompt, yes),
        Action::Commit => commit::run(configured()?, yes),
        Action::Pr {
            draft,
            closes,
            base,
        } => pr::run(configured()?, draft, closes, base, yes),
        Action::Sync => configured().and_then(|_| sync::run(yes)),
        Action::Merge { keep_branch, admin } => {
            configured().and_then(|_| merge::run(false, keep_branch, admin, yes))
        }
        Action::Squash { keep_branch, admin } => {
            configured().and_then(|_| merge::run(true, keep_branch, admin, yes))
        }
    }
}

/// Runs a program and returns its trimmed stdout, or fails with its stderr.
pub fn run(program: &str, args: &[&str], envs: &[(&str, &OsStr)]) -> anyhow::Result<String> {
    let output = Command::new(program)
        .args(args)
        .envs(envs.iter().copied())
        .output()
        .map_err(|error| anyhow::anyhow!("failed to run {program}: {error}"))?;

    if !output.status.success() {
        let command = format!("{program} {} failed", args.join(" "));
        match String::from_utf8_lossy(&output.stderr).trim() {
            "" => anyhow::bail!(command),
            detail => anyhow::bail!("{command}: {detail}"),
        }
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
