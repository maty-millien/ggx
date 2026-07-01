use crate::commands::merge::github;
use crate::tui;
use std::time::Instant;

pub fn run(target: Option<String>, keep_branch: bool, admin: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    ensure_clean_worktree()?;
    let pull_request = github::pull_request(target.as_deref())?;

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
        tui::warning("Aborted");
        return Ok(());
    }

    tui::spinner("Merging pull request", || {
        github::merge(target.as_deref(), keep_branch, admin)
    })?;
    tui::success("Merged PR", &format!("#{}", pull_request.number));

    tui::rail();
    tui::spinner("Syncing base branch", || {
        checkout(&pull_request.base)?;
        pull()?;
        fetch()
    })?;
    tui::success("Synced", &pull_request.base);

    Ok(())
}

fn ensure_clean_worktree() -> anyhow::Result<()> {
    let status = crate::git::run(&["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        anyhow::bail!("Working tree is not clean. Commit or stash your changes first.");
    }

    Ok(())
}

fn fetch() -> anyhow::Result<()> {
    crate::git::run(&["fetch", "--all", "--prune"])?;

    Ok(())
}

fn pull() -> anyhow::Result<()> {
    crate::git::run(&["pull", "--ff-only"])?;

    Ok(())
}

fn checkout(branch: &str) -> anyhow::Result<()> {
    crate::git::run(&["checkout", branch])?;

    Ok(())
}

fn value_or_unknown(value: &str) -> &str {
    if value.is_empty() { "unknown" } else { value }
}

fn summary(pull_request: &github::PullRequest) -> String {
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
