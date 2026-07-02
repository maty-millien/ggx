use crate::tui;
use crate::vcs::git::{self, LocalBranch};
use std::collections::BTreeSet;
use std::time::Instant;

pub fn run() -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_clean_worktree()?;
    let starting_branch = git::current_branch_name()?;

    tui::spinner("Fetching remotes", git::fetch_all_prune)?;
    let base = git::default_base()?;

    tui::spinner("Syncing base branch", || {
        git::checkout(&base)?;
        git::pull_ff_only()
    })?;

    let candidates = cleanup_candidates(
        &git::merged_branches(&base)?,
        &git::local_branches()?,
        &base,
        &starting_branch,
    );

    tui::step("Sync complete", started.elapsed());

    if candidates.is_empty() {
        tui::warning("No local branches to clean");
    } else {
        tui::section("Branches");
        tui::block(&candidates.join("\n"));

        if tui::confirm(&format!(
            "Delete {} local {}?",
            candidates.len(),
            branch_label(candidates.len())
        ))? {
            for branch in &candidates {
                tui::spinner("Deleting branch", || git::delete_branch(branch))?;
                tui::success("Deleted", branch);
            }
        } else {
            tui::warning("Cleanup skipped");
        }
    }

    if starting_branch != base {
        tui::rail();
        tui::spinner("Restoring branch", || git::checkout(&starting_branch))?;
        tui::success("Checked out", &starting_branch);
    }

    Ok(())
}

fn branch_label(count: usize) -> &'static str {
    if count == 1 { "branch" } else { "branches" }
}

fn cleanup_candidates(
    merged_branches: &[String],
    local_branches: &[LocalBranch],
    base: &str,
    starting_branch: &str,
) -> Vec<String> {
    let mut candidates = BTreeSet::new();

    for branch in merged_branches {
        add_candidate(&mut candidates, branch, base, starting_branch);
    }

    for branch in local_branches {
        if is_gone_without_ahead(&branch.upstream_status) {
            add_candidate(&mut candidates, &branch.name, base, starting_branch);
        }
    }

    candidates.into_iter().collect()
}

fn add_candidate(
    candidates: &mut BTreeSet<String>,
    branch: &str,
    base: &str,
    starting_branch: &str,
) {
    if branch != base && branch != starting_branch {
        candidates.insert(branch.to_string());
    }
}

fn is_gone_without_ahead(status: &str) -> bool {
    status.contains("gone") && !status.contains("ahead")
}

#[cfg(test)]
mod tests {
    use super::cleanup_candidates;
    use crate::vcs::git::LocalBranch;

    fn branch(name: &str, upstream_status: &str) -> LocalBranch {
        LocalBranch {
            name: name.to_string(),
            upstream_status: upstream_status.to_string(),
        }
    }

    #[test]
    fn cleanup_candidates_excludes_base_branch() {
        let candidates = cleanup_candidates(
            &["main".to_string(), "feature".to_string()],
            &[],
            "main",
            "topic",
        );

        assert_eq!(candidates, ["feature"]);
    }

    #[test]
    fn cleanup_candidates_excludes_starting_branch() {
        let candidates = cleanup_candidates(
            &[
                "main".to_string(),
                "topic".to_string(),
                "feature".to_string(),
            ],
            &[],
            "main",
            "topic",
        );

        assert_eq!(candidates, ["feature"]);
    }

    #[test]
    fn cleanup_candidates_includes_merged_local_branches() {
        let candidates = cleanup_candidates(
            &["feature".to_string(), "fix".to_string()],
            &[],
            "main",
            "topic",
        );

        assert_eq!(candidates, ["feature", "fix"]);
    }

    #[test]
    fn cleanup_candidates_includes_gone_branches_without_ahead_commits() {
        let candidates = cleanup_candidates(
            &[],
            &[branch("old", "[gone]"), branch("current", "[gone]")],
            "main",
            "current",
        );

        assert_eq!(candidates, ["old"]);
    }

    #[test]
    fn cleanup_candidates_skips_gone_branches_with_ahead_commits() {
        let candidates = cleanup_candidates(
            &[],
            &[
                branch("old", "[gone]"),
                branch("work", "[gone, ahead 1]"),
                branch("feature", "[ahead 2]"),
            ],
            "main",
            "topic",
        );

        assert_eq!(candidates, ["old"]);
    }
}
