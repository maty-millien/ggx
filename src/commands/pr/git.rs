use crate::tui;

pub fn current_branch() -> anyhow::Result<String> {
    let branch = crate::git::run(&["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();

    if branch == "HEAD" {
        anyhow::bail!("Cannot create a PR from detached HEAD.");
    }

    Ok(branch)
}

pub fn default_base() -> anyhow::Result<String> {
    if let Ok(output) = crate::git::run(&["symbolic-ref", "refs/remotes/origin/HEAD"]) {
        let base = output
            .trim()
            .strip_prefix("refs/remotes/origin/")
            .unwrap_or(output.trim())
            .to_string();
        if !base.is_empty() {
            return Ok(base);
        }
    }

    let output = crate::git::run(&["remote", "show", "origin"])?;
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix("HEAD branch: "))
        .map(str::to_string)
        .filter(|base| !base.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Could not detect origin default branch. Pass --base."))
}

pub fn base_ref(base: &str) -> anyhow::Result<String> {
    if crate::git::run(&["rev-parse", "--verify", base]).is_ok() {
        return Ok(base.to_string());
    }

    let remote_base = format!("origin/{}", base);
    if crate::git::run(&["rev-parse", "--verify", &remote_base]).is_ok() {
        return Ok(remote_base);
    }

    anyhow::bail!("Base branch '{}' was not found locally or on origin.", base);
}

pub fn upstream() -> anyhow::Result<String> {
    crate::git::run(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .map(|output| output.trim().to_string())
        .map_err(|_| {
            anyhow::anyhow!(
                "Current branch has no upstream. Push it first with git push -u origin HEAD."
            )
        })
}

pub fn push(upstream: &str) -> anyhow::Result<()> {
    tui::spinner("Pushing branch", || crate::git::run(&["push"]))?;
    tui::success("Pushed to", upstream);

    Ok(())
}
