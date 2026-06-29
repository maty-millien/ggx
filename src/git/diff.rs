use crate::git::run;

pub fn staged_files() -> anyhow::Result<String> {
    let output = run(&["diff", "--staged", "--name-status"])?;

    Ok(output.stdout.trim().to_string())
}

pub fn staged_diff_stat() -> anyhow::Result<String> {
    let output = run(&["diff", "--staged", "--stat"])?;

    Ok(output.stdout.trim().to_string())
}

pub fn staged_diff() -> anyhow::Result<String> {
    let output = run(&["diff", "--staged", "--unified=3"])?;

    Ok(output.stdout.trim().to_string())
}
