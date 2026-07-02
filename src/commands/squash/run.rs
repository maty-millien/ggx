use crate::tui;
use crate::vcs::{git, github};
use std::time::Instant;

pub fn run(keep_branch: bool, admin: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_clean_worktree()?;
    let pull_request = github::pull_request(None)?;

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
        github::squash(None, keep_branch, admin)
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

#[cfg(test)]
mod tests {
    use super::summary;
    use crate::vcs::github::PullRequest;

    fn pull_request() -> PullRequest {
        PullRequest {
            number: "8".to_string(),
            title: "Add squash".to_string(),
            url: "https://github.com/owner/repo/pull/8".to_string(),
            head: "feature".to_string(),
            base: "main".to_string(),
            merge_state: "BLOCKED".to_string(),
            review_decision: String::new(),
        }
    }

    #[test]
    fn summary_renders_core_fields() {
        let output = summary(&pull_request());

        assert!(output.contains("#8 Add squash"));
        assert!(output.contains("https://github.com/owner/repo/pull/8"));
        assert!(output.contains("feature -> main"));
        assert!(output.contains("Merge state: BLOCKED"));
        assert!(!output.contains("Review:"));
    }

    #[test]
    fn summary_uses_unknown_for_empty_merge_state_and_includes_review() {
        let output = summary(&PullRequest {
            merge_state: String::new(),
            review_decision: "CHANGES_REQUESTED".to_string(),
            ..pull_request()
        });

        assert!(output.contains("Merge state: unknown"));
        assert!(output.contains("Review: CHANGES_REQUESTED"));
    }
}
