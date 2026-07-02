use crate::{gh, git, tui};
use std::time::Instant;

pub fn run(keep_branch: bool, admin: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_clean_worktree()?;
    let pull_request = gh::pull_request(None)?;

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
        tui::warning("Aborted");
        return Ok(());
    }

    tui::spinner("Squash merging pull request", || {
        gh::squash(None, keep_branch, admin)
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

fn value_or_unknown(value: &str) -> &str {
    if value.is_empty() { "unknown" } else { value }
}

fn summary(pull_request: &gh::PullRequest) -> String {
    let mut lines = vec![
        format!("#{} {}", pull_request.number, pull_request.title),
        pull_request.url.clone(),
        format!("{} -> {}", pull_request.head, pull_request.base),
        format!(
            "Merge state: {}",
            value_or_unknown(&pull_request.merge_state)
        ),
    ];

    if !pull_request.review_decision.is_empty() {
        lines.push(format!("Review: {}", pull_request.review_decision));
    }

    lines.join("\n")
}
