use crate::commands::{merge::github, merge_common as git};
use crate::tui;
use std::time::Instant;

pub fn run(target: Option<String>, keep_branch: bool, admin: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    let pull_request = github::pull_request(target.as_deref())?;

    tui::step("Pull request found", started.elapsed());
    tui::section("Pull Request");
    tui::block(&format!(
        "#{} {}\n{}\n{} -> {}\nMerge state: {}\nReview: {}",
        pull_request.number,
        pull_request.title,
        pull_request.url,
        pull_request.head,
        pull_request.base,
        value_or_unknown(&pull_request.merge_state),
        value_or_unknown(&pull_request.review_decision)
    ));

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
        tui::warning("Aborted");
        return Ok(());
    }

    tui::spinner("Merging pull request", || {
        github::merge(target.as_deref(), keep_branch, admin)
    })?;
    tui::success("Merged PR", &format!("#{}", pull_request.number));

    tui::rail();
    tui::spinner("Syncing base branch", || {
        git::checkout(&pull_request.base)?;
        git::pull()?;
        git::fetch()
    })?;
    tui::success("Synced", &pull_request.base);

    Ok(())
}

fn value_or_unknown(value: &str) -> &str {
    if value.is_empty() { "unknown" } else { value }
}
