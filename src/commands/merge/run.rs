use super::summary::summary;
use crate::tui;
use crate::vcs::{git, github};
use std::time::Instant;

pub fn run(squash: bool, keep_branch: bool, admin: bool, yes: bool) -> anyhow::Result<()> {
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
    let action = if squash { "Squash merge" } else { "Merge" };
    if !tui::confirm(
        yes,
        &format!(
            "{} PR #{} into {} and {}{}?",
            action, pull_request.number, pull_request.base, cleanup, admin_label
        ),
    )? {
        tui::aborted();
        return Ok(());
    }

    let (progress, done) = if squash {
        ("Squash merging pull request", "Squash merged PR")
    } else {
        ("Merging pull request", "Merged PR")
    };
    tui::spinner(progress, || github::merge(squash, keep_branch, admin))?;
    tui::success(done, &format!("#{}", pull_request.number));

    tui::rail();
    tui::spinner("Syncing base branch", || {
        git::checkout(&pull_request.base)?;
        git::pull_ff_only()?;
        git::fetch_all_prune()
    })?;
    tui::success("Synced", &pull_request.base);

    Ok(())
}
