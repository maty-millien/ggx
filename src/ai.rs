use std::io::Write;
use std::process::{Command, Stdio};

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    let mut child = Command::new("codex")
        .args([
            "exec",
            "--ephemeral",
            "--sandbox",
            "read-only",
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
    stdin.write_all(prompt.as_bytes())?;

    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        anyhow::bail!("codex exec failed: {}", stderr);
    }

    Ok(stdout)
}
