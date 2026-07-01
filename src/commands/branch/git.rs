pub fn branch_exists(name: &str) -> anyhow::Result<bool> {
    let local = crate::git::run(&["branch", "--list", name])?;
    if has_branch_output(&local) {
        return Ok(true);
    }

    if crate::git::run(&["remote", "get-url", "origin"]).is_err() {
        return Ok(false);
    }

    let remote = crate::git::run(&["ls-remote", "--heads", "origin", name])?;
    Ok(has_branch_output(&remote))
}

pub fn create(name: &str) -> anyhow::Result<()> {
    crate::git::run(&["checkout", "-b", name])?;

    Ok(())
}

fn has_branch_output(output: &str) -> bool {
    !output.trim().is_empty()
}
