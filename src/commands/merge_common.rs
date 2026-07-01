const PROTECTED_BRANCHES: &[&str] = &["main", "master", "develop", "dev", "release"];

pub fn current_branch() -> anyhow::Result<String> {
    let branch = crate::git::run(&["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();

    if branch == "HEAD" {
        anyhow::bail!("Cannot run this command from detached HEAD.");
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
        .ok_or_else(|| anyhow::anyhow!("Could not detect origin default branch."))
}

pub fn branch_ref(branch: &str) -> anyhow::Result<String> {
    if crate::git::run(&["rev-parse", "--verify", branch]).is_ok() {
        return Ok(branch.to_string());
    }

    let remote_branch = format!("origin/{}", branch);
    if crate::git::run(&["rev-parse", "--verify", &remote_branch]).is_ok() {
        return Ok(remote_branch);
    }

    anyhow::bail!("Branch '{}' was not found locally or on origin.", branch);
}

pub fn upstream() -> Option<String> {
    crate::git::run(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .ok()
        .map(|output| output.trim().to_string())
        .filter(|output| !output.is_empty())
}

pub fn fetch() -> anyhow::Result<()> {
    crate::git::run(&["fetch", "--all", "--prune"])?;

    Ok(())
}

pub fn pull() -> anyhow::Result<()> {
    crate::git::run(&["pull", "--ff-only"])?;

    Ok(())
}

pub fn checkout(branch: &str) -> anyhow::Result<()> {
    crate::git::run(&["checkout", branch])?;

    Ok(())
}

pub fn force_push_with_lease() -> anyhow::Result<()> {
    crate::git::run(&["push", "--force-with-lease"])?;

    Ok(())
}

pub fn is_protected(branch: &str, target: &str) -> bool {
    branch == target || PROTECTED_BRANCHES.contains(&branch)
}

pub fn ensure_rewrite_allowed(branch: &str, target: &str, admin: bool) -> anyhow::Result<()> {
    if is_protected(branch, target) && !admin {
        anyhow::bail!(
            "Refusing to rewrite protected branch '{}'. Re-run with --admin to unlock this.",
            branch
        );
    }

    Ok(())
}
