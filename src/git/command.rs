use std::process::Command;

pub struct Output {
    pub stdout: String,
    pub stderr: String,
}

pub fn run(args: &[&str]) -> anyhow::Result<Output> {
    let output = Command::new("git").args(args).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(Output { stdout, stderr })
    } else {
        anyhow::bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
}
