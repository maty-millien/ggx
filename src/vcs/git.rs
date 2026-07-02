use std::process::{Command, Stdio};

pub struct LocalBranch {
    pub name: String,
    pub upstream_status: String,
}

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

pub fn upstream() -> anyhow::Result<String> {
    run(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .map(|output| output.trim().to_string())
        .map_err(|_| {
            anyhow::anyhow!(
                "Current branch has no upstream. Push it first with git push -u origin HEAD."
            )
        })
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
    run(&["branch", "-d", name])?;

    Ok(())
}

fn has_output(output: &str) -> bool {
    !output.trim().is_empty()
}

fn parse_branch_names(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_local_branches(output: &str) -> Vec<LocalBranch> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            let (name, upstream_status) = line.split_once('\t').unwrap_or((line, ""));

            Some(LocalBranch {
                name: name.trim().to_string(),
                upstream_status: upstream_status.trim().to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{has_output, parse_branch_names, parse_local_branches};

    #[test]
    fn has_output_rejects_empty_or_whitespace() {
        assert!(!has_output(""));
        assert!(!has_output(" \n\t "));
    }

    #[test]
    fn has_output_accepts_non_whitespace() {
        assert!(has_output("main\n"));
    }

    #[test]
    fn parse_branch_names_omits_empty_lines() {
        assert_eq!(
            parse_branch_names("main\n\nfeature\n"),
            vec!["main".to_string(), "feature".to_string()]
        );
    }

    #[test]
    fn parse_local_branches_reads_optional_upstream_status() {
        let branches = parse_local_branches("main\t\nfeature\t[gone]\n");

        assert_eq!(branches[0].name, "main");
        assert_eq!(branches[0].upstream_status, "");
        assert_eq!(branches[1].name, "feature");
        assert_eq!(branches[1].upstream_status, "[gone]");
    }
}
