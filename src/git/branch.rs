use crate::git::run;

pub fn current_branch() -> anyhow::Result<String> {
    let output = run(&["rev-parse", "--abbrev-ref", "HEAD"])?;

    Ok(output.stdout.trim().to_string())
}

pub fn repo_root() -> anyhow::Result<String> {
    let output = run(&["rev-parse", "--show-toplevel"])?;

    Ok(output.stdout.trim().to_string())
}
