#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    Branch {
        prompt: Option<String>,
    },
    Commit,
    Pr {
        #[arg(long)]
        draft: bool,
        #[arg(long)]
        base: Option<String>,
        #[arg(long = "closes")]
        closes: Vec<String>,
    },
    Merge {
        target: Option<String>,
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
