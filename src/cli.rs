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
    Branch {
        prompt: Option<String>,
    },
    Commit,
    Pr {
        #[arg(long)]
        draft: bool,
        #[arg(long = "closes")]
        closes: Vec<String>,
    },
    Sync,
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

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use clap::Parser;

    #[test]
    fn parses_branch_prompt() {
        let cli = Cli::parse_from(["ggx", "branch", "new thing"]);

        match cli.command {
            Some(Command::Branch { prompt }) => assert_eq!(prompt.as_deref(), Some("new thing")),
            _ => panic!("expected branch command"),
        }
    }

    #[test]
    fn parses_pr_options() {
        let cli = Cli::parse_from(["ggx", "pr", "--draft", "--closes", "#1", "--closes", "#2"]);

        match cli.command {
            Some(Command::Pr { draft, closes }) => {
                assert!(draft);
                assert_eq!(closes, ["#1", "#2"]);
            }
            _ => panic!("expected pr command"),
        }
    }

    #[test]
    fn parses_sync_command() {
        let cli = Cli::parse_from(["ggx", "sync"]);

        assert!(!cli.version);
        match cli.command {
            Some(Command::Sync) => {}
            _ => panic!("expected sync command"),
        }
    }

    #[test]
    fn parses_merge_options() {
        let cli = Cli::parse_from(["ggx", "merge", "12", "--keep-branch", "--admin"]);

        match cli.command {
            Some(Command::Merge {
                target,
                keep_branch,
                admin,
            }) => {
                assert_eq!(target.as_deref(), Some("12"));
                assert!(keep_branch);
                assert!(admin);
            }
            _ => panic!("expected merge command"),
        }
    }

    #[test]
    fn parses_squash_options() {
        let cli = Cli::parse_from(["ggx", "squash", "--keep-branch", "--admin"]);

        match cli.command {
            Some(Command::Squash { keep_branch, admin }) => {
                assert!(keep_branch);
                assert!(admin);
            }
            _ => panic!("expected squash command"),
        }
    }

    #[test]
    fn parses_long_version_flag() {
        let cli = Cli::parse_from(["ggx", "--version"]);

        assert!(cli.version);
        assert!(cli.command.is_none());
    }

    #[test]
    fn parses_short_version_flag() {
        let cli = Cli::parse_from(["ggx", "-v"]);

        assert!(cli.version);
        assert!(cli.command.is_none());
    }
}
