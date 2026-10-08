use crate::{git, tui};
use std::time::Instant;

pub fn run(yes: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_clean_worktree()?;
    let starting_branch = git::run(&["symbolic-ref", "--short", "HEAD"])?;

    tui::spinner("Fetching remotes", || {
        git::run(&["fetch", "--all", "--prune"])
    })?;
    let base = git::default_base()?;
    tui::spinner("Syncing base branch", || {
        git::run(&["checkout", &base])?;
        git::run(&["pull", "--ff-only"])
    })?;

    let mut candidates: Vec<String> = git::merged_branches(&base)?
        .into_iter()
        .filter(|branch| *branch != starting_branch)
        .chain(git::gone_branches()?)
        .filter(|branch| *branch != base)
        .collect();
    candidates.sort();
    candidates.dedup();
    let mut deleted_starting_branch = false;

    tui::step("Sync complete", started.elapsed());

    if candidates.is_empty() {
        tui::warning("No local branches to clean");
    } else {
        tui::section("Branches");
        tui::block(&candidates.join("\n"));

        let noun = if candidates.len() == 1 {
            "branch"
        } else {
            "branches"
        };
        if tui::confirm(yes, &format!("Delete {} local {noun}?", candidates.len()))? {
            for branch in &candidates {
                tui::spinner("Deleting branch", || git::run(&["branch", "-D", branch]))?;
                tui::success("Deleted", branch);
            }
            deleted_starting_branch = candidates.contains(&starting_branch);
        } else {
            tui::warning("Cleanup skipped");
        }
    }

    if starting_branch != base && !deleted_starting_branch {
        tui::rail();
        tui::spinner("Restoring branch", || {
            git::run(&["checkout", &starting_branch])
        })?;
        tui::success("Checked out", &starting_branch);
    }

    Ok(())
}
