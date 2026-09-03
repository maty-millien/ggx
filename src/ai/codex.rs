use std::io::Write;
use std::process::{Command, Stdio};

const MODEL: &str = "gpt-5.6-luna";
const REASONING_EFFORT: &str = "model_reasoning_effort=\"low\"";
const DIRECT_RESPONSE_INSTRUCTIONS: &str = r#"Do not invoke tools.
Do not inspect files.
Do not run commands.
Return only the requested text and nothing else.

"#;

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    let mut child = codex_command().spawn()?;

    let stdin = child
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("failed to open codex stdin"))?;
    stdin.write_all(direct_response_prompt(prompt).as_bytes())?;

    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        anyhow::bail!("codex exec failed: {}", stderr);
    }

    Ok(stdout)
}

fn codex_command() -> Command {
    let mut command = Command::new("codex");
    command
        .args([
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
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command
}

fn direct_response_prompt(prompt: &str) -> String {
    format!("{}{}", DIRECT_RESPONSE_INSTRUCTIONS, prompt)
}

#[cfg(test)]
mod tests {
    use super::{MODEL, REASONING_EFFORT, codex_command, direct_response_prompt};
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

    #[test]
    fn prefixes_generation_prompt_with_direct_response_instructions() {
        assert_eq!(
            direct_response_prompt("name a branch"),
            "Do not invoke tools.\nDo not inspect files.\nDo not run commands.\nReturn only the requested text and nothing else.\n\nname a branch"
        );
    }
}
