use std::io::Write;
use std::process::{Command, Stdio};

const DIRECT_RESPONSE_INSTRUCTIONS: &str = r#"Do not invoke tools.
Do not inspect files.
Do not run commands.
Return only the requested text.

"#;

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    let mut child = Command::new("codex")
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
            "gpt-5.4-mini",
            "-c",
            "model_reasoning_effort=\"low\"",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdin = child
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("failed to open codex stdin"))?;
    stdin.write_all(format!("{}{}", DIRECT_RESPONSE_INSTRUCTIONS, prompt).as_bytes())?;

    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        anyhow::bail!("codex exec failed: {}", stderr);
    }

    Ok(stdout)
}
