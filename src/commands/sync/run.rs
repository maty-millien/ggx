use super::candidates::{branch_label, cleanup_candidates};
use crate::tui;
use crate::vcs::git;
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
