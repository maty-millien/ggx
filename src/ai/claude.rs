use super::{Provider, run};
use std::process::Command;

const MODEL: &str = "haiku";

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    run(Provider::Claude, claude_command(), prompt)
}

fn claude_command() -> Command {
    let mut command = Command::new("claude");
    command.args([
        "--print",
        "--output-format",
        "text",
        "--safe-mode",
        "--tools",
        "",
        "--no-session-persistence",
        "--no-chrome",
        "--model",
        MODEL,
    ]);
    command
}

#[cfg(test)]
mod tests {
    use super::{MODEL, claude_command};
    use std::ffi::OsStr;

    #[test]
    fn builds_non_interactive_claude_command() {
        let command = claude_command();
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let expected = [
            "--print",
            "--output-format",
            "text",
            "--safe-mode",
            "--tools",
            "",
            "--no-session-persistence",
            "--no-chrome",
            "--model",
            MODEL,
        ]
        .map(String::from);

        assert_eq!(command.get_program(), OsStr::new("claude"));
        assert_eq!(args, expected);
    }
}
