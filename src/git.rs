use anyhow::{Context, ensure};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

pub fn executable() -> &'static OsStr {
    static EXECUTABLE: OnceLock<PathBuf> = OnceLock::new();
    EXECUTABLE
        .get_or_init(|| {
            Command::new("git")
                .arg("--exec-path")
                .output()
                .ok()
                .map(|output| {
                    PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()).join("git")
                })
                .filter(|path| path.is_file())
                .unwrap_or_else(|| PathBuf::from("git"))
        })
        .as_os_str()
}

pub fn run(args: &[&str]) -> anyhow::Result<String> {
    crate::run("git", args, &[])
}

pub fn has_changes() -> anyhow::Result<bool> {
    Ok(!run(&["status", "--porcelain"])?.is_empty())
}

pub fn ensure_clean_worktree() -> anyhow::Result<()> {
    ensure!(
        !has_changes()?,
        "Working tree is not clean. Commit or stash your changes first."
    );
    Ok(())
}

pub fn ensure_no_conflicts() -> anyhow::Result<()> {
    ensure!(
        run(&["ls-files", "--unmerged"])?.is_empty(),
        "Resolve conflicts before committing."
    );
    Ok(())
}

pub fn has_origin_remote() -> bool {
    run(&["remote", "get-url", "origin"]).is_ok()
}

pub fn optional_upstream() -> Option<String> {
    run(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]).ok()
}

pub fn branch_names() -> anyhow::Result<Vec<String>> {
    let mut names: Vec<String> = run(&["branch", "--format", "%(refname:short)"])?
        .lines()
        .map(str::to_string)
        .collect();
    if has_origin_remote() {
        names.extend(
            run(&["ls-remote", "--heads", "origin"])?
                .lines()
                .filter_map(|line| Some(line.split_once("\trefs/heads/")?.1.to_string())),
        );
    }
    Ok(names)
}

pub fn default_base() -> anyhow::Result<String> {
    if let Ok(head) = run(&["symbolic-ref", "refs/remotes/origin/HEAD"])
        && let Some(base) = head.strip_prefix("refs/remotes/origin/")
    {
        return Ok(base.to_string());
    }

    run(&["remote", "show", "origin"])?
        .lines()
        .find_map(|line| line.trim().strip_prefix("HEAD branch: "))
        .filter(|base| !base.is_empty())
        .map(str::to_string)
        .context("Could not detect origin default branch.")
}

pub fn base_ref(base: &str) -> anyhow::Result<String> {
    let remote = format!("origin/{base}");
    [base, &remote]
        .into_iter()
        .find(|candidate| run(&["rev-parse", "--verify", candidate]).is_ok())
        .map(str::to_string)
        .with_context(|| format!("Base branch '{base}' was not found locally or on origin."))
}

pub fn merged_branches(base: &str) -> anyhow::Result<Vec<String>> {
    let output = run(&["branch", "--merged", base, "--format", "%(refname:short)"])?;
    Ok(output.lines().map(str::to_string).collect())
}

pub fn gone_branches() -> anyhow::Result<Vec<String>> {
    let output = run(&["branch", "--format", "%(refname:short)%09%(upstream:track)"])?;
    Ok(output
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(_, track)| track.contains("gone") && !track.contains("ahead"))
        .map(|(name, _)| name.to_string())
        .collect())
}
