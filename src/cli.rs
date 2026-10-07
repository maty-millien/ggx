#[derive(clap::Parser)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    #[arg(short = 'v', long = "version")]
    pub version: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand)]
pub enum Command {
    Setup {
        #[arg(long, value_parser = ["codex", "claude", "copilot"])]
        provider: Option<String>,
    },
    Branch {
        prompt: Option<String>,
    },
    Commit,
    Pr {
        #[arg(long)]
        draft: bool,
        #[arg(long = "closes")]
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
