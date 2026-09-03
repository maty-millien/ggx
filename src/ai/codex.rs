use super::{Provider, run};
use std::process::Command;

const MODEL: &str = "gpt-5.6-luna";
const REASONING_EFFORT: &str = "model_reasoning_effort=\"low\"";
pub fn generate(prompt: &str) -> anyhow::Result<String> {
    run(Provider::Codex, codex_command(), prompt)
}

fn codex_command() -> Command {
    let mut command = Command::new("codex");
    command.args([
        "exec",
        "--ephemeral",
        "--ignore-user-config",
        "--sandbox",
        "read-only",
        "--disable",
        "apps",
        "--disable",
        "browser_use",
        "--disable",
        "computer_use",
        "--disable",
        "goals",
        "--disable",
        "image_generation",
        "--disable",
        "multi_agent",
        "--disable",
        "shell_tool",
        "--disable",
        "workspace_dependencies",
        "--model",
        MODEL,
        "-c",
        REASONING_EFFORT,
        "-",
    ]);

    command
}

#[cfg(test)]
mod tests {
    use super::{MODEL, REASONING_EFFORT, codex_command};
    use std::ffi::OsStr;

    #[test]
    fn builds_non_interactive_codex_command() {
        let command = codex_command();
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let expected = [
            "exec",
            "--ephemeral",
            "--ignore-user-config",
            "--sandbox",
            "read-only",
            "--disable",
            "apps",
            "--disable",
            "browser_use",
            "--disable",
            "computer_use",
            "--disable",
            "goals",
            "--disable",
            "image_generation",
            "--disable",
            "multi_agent",
            "--disable",
            "shell_tool",
            "--disable",
            "workspace_dependencies",
            "--model",
            MODEL,
            "-c",
            REASONING_EFFORT,
            "-",
        ]
        .map(String::from);

        assert_eq!(command.get_program(), OsStr::new("codex"));
        assert_eq!(args, expected);
    }
}
