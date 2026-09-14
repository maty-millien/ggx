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

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use clap::Parser;

    #[test]
    fn requires_subcommand_or_version() {
        assert!(Cli::try_parse_from(["ggx"]).is_err());
        assert!(Cli::try_parse_from(["ggx", "unknown"]).is_err());
    }

    #[test]
    fn parses_optional_branch_prompt() {
        let Some(Command::Branch { prompt }) =
            Cli::parse_from(["ggx", "branch", "new thing"]).command
        else {
            panic!("expected branch command");
        };
        assert_eq!(prompt.as_deref(), Some("new thing"));

        let Some(Command::Branch { prompt }) = Cli::parse_from(["ggx", "branch"]).command else {
            panic!("expected branch command");
        };
        assert!(prompt.is_none());
    }

    #[test]
    fn parses_pr_options() {
        let cli = Cli::parse_from([
            "ggx", "pr", "--draft", "--closes", "#1", "--closes", "#2", "--base", "dev",
        ]);

        match cli.command {
            Some(Command::Pr {
                draft,
                closes,
                base,
            }) => {
                assert!(draft);
                assert_eq!(closes, ["#1", "#2"]);
                assert_eq!(base.as_deref(), Some("dev"));
            }
            _ => panic!("expected pr command"),
        }
    }

    #[test]
    fn pr_options_default_to_off() {
        let cli = Cli::parse_from(["ggx", "pr"]);

        match cli.command {
            Some(Command::Pr {
                draft,
                closes,
                base,
            }) => {
                assert!(!draft);
                assert!(closes.is_empty());
                assert!(base.is_none());
            }
            _ => panic!("expected pr command"),
        }
    }

    #[test]
    fn parses_merge_and_squash_flags() {
        let Some(Command::Merge { keep_branch, admin }) =
            Cli::parse_from(["ggx", "merge", "--keep-branch", "--admin"]).command
        else {
            panic!("expected merge command");
        };
        assert!(keep_branch && admin);

        let Some(Command::Squash { keep_branch, admin }) =
            Cli::parse_from(["ggx", "squash"]).command
        else {
            panic!("expected squash command");
        };
        assert!(!keep_branch && !admin);
    }

    #[test]
    fn parses_version_flags() {
        for flag in ["--version", "-v"] {
            let cli = Cli::parse_from(["ggx", flag]);

            assert!(cli.version);
            assert!(cli.command.is_none());
        }
    }
}
