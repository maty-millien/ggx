use crate::git::run;
use std::process::Command;

pub fn current_branch() -> anyhow::Result<String> {
    let output = run(&["rev-parse", "--abbrev-ref", "HEAD"])?;

    Ok(output.stdout.trim().to_string())
}

pub fn repo_root() -> anyhow::Result<String> {
    let output = run(&["rev-parse", "--show-toplevel"])?;

    Ok(output.stdout.trim().to_string())
}

pub fn upstream_branch() -> anyhow::Result<Option<String>> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    ))
}
