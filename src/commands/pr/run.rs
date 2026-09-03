use crate::commands::commit;
use crate::commands::generation::{self, Request};
use crate::commands::pr::context::Context;
use crate::tui;
use crate::vcs::{changes, git, github};
use std::time::Instant;

pub fn run(draft: bool, closes: Vec<String>, requested_base: Option<String>) -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_no_conflicts()?;

    let branch = git::current_branch()?;
    let default_base = git::default_base()?;
    let base = requested_base.unwrap_or_else(|| default_base.clone());
    let has_changes = git::has_changes()?;
    let create_branch = workflow(&branch, &base, &default_base, has_changes)?;

    if !create_branch && let Some(pull_request) = github::open_pull_request(&branch)? {
        anyhow::bail!(
            "A pull request is already open for this branch: {}",
            pull_request.url
        );
    }

    let mut context = Context::collect(branch.clone(), base.clone(), closes)?;
    tui::step("Analysis complete", started.elapsed());
    show_changes(&context);

    let generation_context = generation::Context {
        current_branch: branch.clone(),
        base: Some(base.clone()),
        user_prompt: None,
        commits: context.commits.clone(),
        committed_files: context.files.clone(),
        committed_stat: context.stat.clone(),
        committed_diff: context.diff.clone(),
        pending: context.pending.as_ref().map(Into::into),
        issues: std::mem::take(&mut context.issues),
    };
    let needs_commit = context.pending.is_some();
    let (generated, elapsed) = tui::timed_spinner("Generating pull request workflow", || {
        generation::generate(
            &generation_context,
            Request {
                branch: create_branch,
                commit: needs_commit,
                pull_request: true,
            },
        )
    })?;
    let pull_request = generated
        .pull_request
        .expect("generation requires pull request");
    let head = generated.branch.as_deref().unwrap_or(&branch);

    tui::step("Workflow generated", elapsed);
    if let Some(generated_branch) = &generated.branch {
        tui::section("Branch");
        tui::message(generated_branch);
    }
    if let Some(message) = &generated.commit {
        tui::section("Commit");
        tui::message(message);
    }
    tui::section("Title");
    tui::message(&pull_request.title);
    tui::section("Body");
    tui::block(&pull_request.body);

    if !tui::confirm(&action_prompt(create_branch, needs_commit, head, &base))? {
        tui::aborted();
        return Ok(());
    }

    if create_branch {
        tui::spinner("Creating branch", || git::create_branch(head))?;
        tui::success("Checked out", head);
        tui::rail();
    }

    if let Some(mut pending) = context.pending {
        pending.branch = head.to_string();
        let prepared = commit::prepare(
            pending,
            generated.commit.expect("generation requires commit"),
            if create_branch {
                None
            } else {
                git::optional_upstream()
            },
            git::has_origin_remote(),
        );
        commit::finish(&prepared)?;
    } else {
        push_current_branch(head)?;
    }

    tui::rail();
    let url = tui::spinner("Creating pull request", || {
        github::create_pr(&base, head, &pull_request.title, &pull_request.body, draft)
    })?;
    tui::success("Created PR", &url);

    Ok(())
}

fn workflow(
    branch: &str,
    base: &str,
    default_base: &str,
    has_changes: bool,
) -> anyhow::Result<bool> {
    if branch == default_base && base != default_base {
        anyhow::bail!(
            "Current branch '{}' is the default base branch. Checkout '{}' or a feature branch first.",
            branch,
            base
        );
    }

    if branch == base {
        if !has_changes {
            anyhow::bail!("No pending changes found on base branch '{}'.", base);
        }
        return Ok(true);
    }

    Ok(false)
}

fn show_changes(context: &Context) {
    let mut rows = changes::from_files_and_numstat(&context.files, &context.numstat);
    if let Some(pending) = &context.pending {
        rows.extend(changes::from_files_and_numstat(
            &pending.files,
            &pending.numstat,
        ));
    }

    tui::section("Changes");
    tui::change_rows(&rows);
}

fn action_prompt(create_branch: bool, needs_commit: bool, head: &str, base: &str) -> String {
    if create_branch {
        format!(
            "Create {}, commit, push, and create PR into {}?",
            head, base
        )
    } else if needs_commit {
        format!("Commit, push {}, and create PR into {}?", head, base)
    } else {
        format!("Push {} and create PR into {}?", head, base)
    }
}

fn push_current_branch(branch: &str) -> anyhow::Result<()> {
    if let Some(upstream) = git::optional_upstream() {
        tui::spinner("Pushing branch", git::push)?;
        tui::success("Pushed to", &upstream);
    } else {
        let destination = format!("origin/{}", branch);
        tui::spinner("Pushing branch", || git::push_branch(branch))?;
        tui::success("Pushed to", &destination);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{action_prompt, workflow};

    #[test]
    fn creates_branch_from_dirty_selected_base() {
        assert!(workflow("dev", "dev", "main", true).unwrap());
    }

    #[test]
    fn keeps_dirty_feature_branch() {
        assert!(!workflow("feat/api", "dev", "main", true).unwrap());
    }

    #[test]
    fn keeps_clean_feature_branch() {
        assert!(!workflow("feat/api", "dev", "main", false).unwrap());
    }

    #[test]
    fn rejects_clean_selected_base() {
        assert!(workflow("dev", "dev", "main", false).is_err());
    }

    #[test]
    fn rejects_default_base_with_different_requested_base() {
        assert!(workflow("main", "dev", "main", true).is_err());
    }

    #[test]
    fn describes_single_workflow_action() {
        assert_eq!(
            action_prompt(true, true, "feat/api", "dev"),
            "Create feat/api, commit, push, and create PR into dev?"
        );
        assert_eq!(
            action_prompt(false, false, "feat/api", "dev"),
            "Push feat/api and create PR into dev?"
        );
    }
}
