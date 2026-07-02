use crate::tui;
use crate::vcs::{git, github};
use std::time::Instant;

pub fn run(target: Option<String>, keep_branch: bool, admin: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_clean_worktree()?;
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
        tui::aborted();
        return Ok(());
    }

    tui::spinner("Merging pull request", || {
        github::merge(target.as_deref(), keep_branch, admin)
    })?;
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
            number: "7".to_string(),
            title: "Add merge".to_string(),
            url: "https://github.com/owner/repo/pull/7".to_string(),
            head: "feature".to_string(),
            base: "main".to_string(),
            merge_state: "CLEAN".to_string(),
            review_decision: String::new(),
        }
    }

    #[test]
    fn summary_renders_core_fields() {
        let output = summary(&pull_request());

        assert!(output.contains("#7 Add merge"));
        assert!(output.contains("https://github.com/owner/repo/pull/7"));
        assert!(output.contains("feature -> main"));
        assert!(output.contains("Merge state: CLEAN"));
        assert!(!output.contains("Review:"));
    }

    #[test]
    fn summary_uses_unknown_for_empty_merge_state_and_includes_review() {
        let output = summary(&PullRequest {
            merge_state: String::new(),
            review_decision: "APPROVED".to_string(),
            ..pull_request()
        });

        assert!(output.contains("Merge state: unknown"));
        assert!(output.contains("Review: APPROVED"));
    }
}
