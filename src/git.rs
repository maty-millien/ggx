use std::process::{Command, Stdio};

pub fn run(args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(args)
        .stderr(Stdio::null())
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        anyhow::bail!("git {} failed", args.join(" "));
    }
}
