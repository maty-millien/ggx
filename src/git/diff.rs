use crate::git::run;

pub fn staged_files() -> anyhow::Result<String> {
    let output = run(&["diff", "--staged", "--name-status"])?;

    Ok(output.stdout.trim().to_string())
}

pub fn working_tree_status() -> anyhow::Result<String> {
    let output = run(&["status", "--short", "--untracked-files=all"])?;

    Ok(output.stdout.trim_end().to_string())
}

pub fn staged_diff_stat() -> anyhow::Result<String> {
    let output = run(&["diff", "--staged", "--stat"])?;

    Ok(output.stdout.trim().to_string())
}

pub fn staged_numstat() -> anyhow::Result<String> {
    let output = run(&["diff", "--staged", "--numstat"])?;

    Ok(output.stdout.trim().to_string())
}

pub fn unstaged_diff_stat() -> anyhow::Result<String> {
    let output = run(&["diff", "--stat"])?;

    Ok(output.stdout.trim().to_string())
}

pub fn unstaged_numstat() -> anyhow::Result<String> {
    let output = run(&["diff", "--numstat"])?;

    Ok(output.stdout.trim().to_string())
}

pub fn staged_diff() -> anyhow::Result<String> {
    let output = run(&["diff", "--staged", "--unified=3"])?;

    Ok(output.stdout.trim().to_string())
}

pub fn unstaged_diff() -> anyhow::Result<String> {
    let output = run(&["diff", "--unified=3"])?;

    Ok(output.stdout.trim().to_string())
}
