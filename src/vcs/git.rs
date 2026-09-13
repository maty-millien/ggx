mod parse;

use parse::{failure_message, has_output, parse_branch_names, parse_local_branches};
use std::ffi::OsStr;
use std::process::Command;

pub struct LocalBranch {
    pub name: String,
    pub upstream_status: String,
}

pub fn run(args: &[&str]) -> anyhow::Result<String> {
    run_with_env(args, &[])
}

pub fn run_with_env(args: &[&str], envs: &[(&str, &OsStr)]) -> anyhow::Result<String> {
    let mut command = Command::new("git");
    command.args(args);

    for (key, value) in envs {
        command.env(key, value);
    }

    let output = command.output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        anyhow::bail!(failure_message(args, &output.stderr));
    }
}

pub fn branch_exists(name: &str) -> anyhow::Result<bool> {
    let local = run(&["branch", "--list", name])?;
    if has_output(&local) {
        return Ok(true);
    }

    if run(&["remote", "get-url", "origin"]).is_err() {
        return Ok(false);
    }

    let remote = run(&["ls-remote", "--heads", "origin", name])?;
    Ok(has_output(&remote))
}

pub fn create_branch(name: &str) -> anyhow::Result<()> {
    run(&["checkout", "-b", name])?;

    Ok(())
}

pub fn push_branch(name: &str) -> anyhow::Result<()> {
    run(&["push", "-u", "origin", name])?;

    Ok(())
}

pub fn current_branch() -> anyhow::Result<String> {
    let branch = run(&["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();

    if branch == "HEAD" {
        anyhow::bail!("Cannot create a PR from detached HEAD.");
    }

    Ok(branch)
}

pub fn current_branch_name() -> anyhow::Result<String> {
    run(&["symbolic-ref", "--short", "HEAD"]).map(|output| output.trim().to_string())
}

pub fn ensure_clean_worktree() -> anyhow::Result<()> {
    let status = run(&["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        anyhow::bail!("Working tree is not clean. Commit or stash your changes first.");
    }

    Ok(())
}

pub fn has_changes() -> anyhow::Result<bool> {
    run(&["status", "--porcelain"]).map(|status| !status.trim().is_empty())
}

pub fn ensure_no_conflicts() -> anyhow::Result<()> {
    let unmerged = run(&["ls-files", "--unmerged"])?;
    if !unmerged.trim().is_empty() {
        anyhow::bail!("Resolve conflicts before committing.");
    }

    Ok(())
}

pub fn default_base() -> anyhow::Result<String> {
    if let Ok(output) = run(&["symbolic-ref", "refs/remotes/origin/HEAD"]) {
        let base = output
            .trim()
            .strip_prefix("refs/remotes/origin/")
            .unwrap_or(output.trim())
            .to_string();
        if !base.is_empty() {
            return Ok(base);
        }
    }

    let output = run(&["remote", "show", "origin"])?;
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix("HEAD branch: "))
        .map(str::to_string)
        .filter(|base| !base.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Could not detect origin default branch."))
}

pub fn base_ref(base: &str) -> anyhow::Result<String> {
    if run(&["rev-parse", "--verify", base]).is_ok() {
        return Ok(base.to_string());
    }

    let remote_base = format!("origin/{}", base);
    if run(&["rev-parse", "--verify", &remote_base]).is_ok() {
        return Ok(remote_base);
    }

    anyhow::bail!("Base branch '{}' was not found locally or on origin.", base);
}

pub fn optional_upstream() -> Option<String> {
    run(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .ok()
        .map(|output| output.trim().to_string())
}

pub fn has_origin_remote() -> bool {
    run(&["remote", "get-url", "origin"]).is_ok()
}

pub fn stage_all() -> anyhow::Result<String> {
    run(&["add", "--all"])
}

pub fn commit(message: &str) -> anyhow::Result<String> {
    run(&["commit", "-m", message])
}

pub fn push() -> anyhow::Result<String> {
    run(&["push"])
}

pub fn fetch_all_prune() -> anyhow::Result<()> {
    run(&["fetch", "--all", "--prune"])?;

    Ok(())
}

pub fn pull_ff_only() -> anyhow::Result<()> {
    run(&["pull", "--ff-only"])?;

    Ok(())
}

pub fn checkout(branch: &str) -> anyhow::Result<()> {
    run(&["checkout", branch])?;

    Ok(())
}

pub fn merged_branches(base: &str) -> anyhow::Result<Vec<String>> {
    let output = run(&["branch", "--merged", base, "--format", "%(refname:short)"])?;

    Ok(parse_branch_names(&output))
}

pub fn local_branches() -> anyhow::Result<Vec<LocalBranch>> {
    let output = run(&["branch", "--format", "%(refname:short)%09%(upstream:track)"])?;

    Ok(parse_local_branches(&output))
}

pub fn delete_branch(name: &str) -> anyhow::Result<()> {
    run(&["branch", "-D", name])?;

    Ok(())
}
