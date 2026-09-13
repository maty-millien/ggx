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
        "Squash merge PR #{} into {} and {}{}?",
        pull_request.number, pull_request.base, cleanup, admin_label
    ))? {
        tui::aborted();
        return Ok(());
    }

    tui::spinner("Squash merging pull request", || {
        github::squash(keep_branch, admin)
    })?;
    tui::success("Squash merged PR", &format!("#{}", pull_request.number));

    tui::rail();
    tui::spinner("Syncing base branch", || {
        git::checkout(&pull_request.base)?;
        git::pull_ff_only()?;
        git::fetch_all_prune()
    })?;
    tui::success("Synced", &pull_request.base);

    Ok(())
}
