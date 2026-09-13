use super::summary::summary;
use crate::tui;
use crate::vcs::{git, github};
use std::time::Instant;

pub fn run(keep_branch: bool, admin: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_clean_worktree()?;
    let pull_request = github::pull_request()?;

    tui::step("Pull request found", started.elapsed());
    tui::section("Pull Request");
    tui::block(&summary(&pull_request));

    let cleanup = if keep_branch {
        "keep branch"
    } else {
        "delete branch"
    };
    let admin_label = if admin { " with admin" } else { "" };
    if !tui::confirm(&format!(
        "Merge PR #{} into {} and {}{}?",
        pull_request.number, pull_request.base, cleanup, admin_label
    ))? {
        tui::aborted();
        return Ok(());
    }

    tui::spinner("Merging pull request", || github::merge(keep_branch, admin))?;
    tui::success("Merged PR", &format!("#{}", pull_request.number));

    tui::rail();
    tui::spinner("Syncing base branch", || {
        git::checkout(&pull_request.base)?;
        git::pull_ff_only()?;
        git::fetch_all_prune()
    })?;
    tui::success("Synced", &pull_request.base);

    Ok(())
}
