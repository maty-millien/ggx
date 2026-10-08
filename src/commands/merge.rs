use crate::{git, github, tui};
use std::time::Instant;

pub fn run(squash: bool, keep_branch: bool, admin: bool, yes: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_clean_worktree()?;
    let pull_request = github::pull_request()?;

    tui::step("Pull request found", started.elapsed());
    tui::section("Pull Request");
    tui::block(&summary(&pull_request));

    let (action, progress, done) = if squash {
        (
            "Squash merge",
            "Squash merging pull request",
            "Squash merged PR",
        )
    } else {
        ("Merge", "Merging pull request", "Merged PR")
    };
    let cleanup = if keep_branch {
        "keep branch"
    } else {
        "delete branch"
    };
    let admin_label = if admin { " with admin" } else { "" };
    let prompt = format!(
        "{action} PR #{} into {} and {cleanup}{admin_label}?",
        pull_request.number, pull_request.base
    );
    if !tui::confirm(yes, &prompt)? {
        tui::aborted();
        return Ok(());
    }

    tui::spinner(progress, || github::merge(squash, keep_branch, admin))?;
    tui::success(done, &format!("#{}", pull_request.number));

    tui::rail();
    tui::spinner("Syncing base branch", || {
        git::run(&["checkout", &pull_request.base])?;
        git::run(&["pull", "--ff-only"])?;
        git::run(&["fetch", "--all", "--prune"])
    })?;
    tui::success("Synced", &pull_request.base);

    Ok(())
}

fn summary(pull_request: &github::PullRequest) -> String {
    let mut lines = vec![
        format!("#{}  {}", pull_request.number, pull_request.title),
        pull_request.url.clone(),
        String::new(),
        format!(
            "{:<12}{} → {}",
            "Branch", pull_request.head, pull_request.base
        ),
        format!(
            "{:<12}{}",
            "Merge state",
            readable(&pull_request.merge_state)
        ),
    ];
    if !pull_request.review_decision.is_empty() {
        lines.push(format!(
            "{:<12}{}",
            "Review",
            readable(&pull_request.review_decision)
        ));
    }

    lines.join("\n")
}

fn readable(status: &str) -> String {
    let status = if status.is_empty() { "unknown" } else { status };
    let status = status.replace('_', " ").to_lowercase();
    format!("{}{}", status[..1].to_uppercase(), &status[1..])
}
