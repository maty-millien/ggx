use crate::commands::{
    branch::context::Context,
    commit::{self, context::Context as CommitContext},
    generation::{self, Request},
};
use crate::tui;
use crate::vcs::git;
use std::time::Instant;

pub fn run(input_prompt: Option<String>) -> anyhow::Result<()> {
    let started = Instant::now();
    let context = Context::collect(input_prompt)?;
    git::ensure_no_conflicts()?;

    let mut commit_context = if context.has_changes() {
        Some(CommitContext::collect_for_branch(context.branch.clone())?)
    } else {
        None
    };

    tui::step("Analysis complete", started.elapsed());
    if let Some(commit_context) = &commit_context {
        commit::show_changes(commit_context);
    }

    let generation_context = generation::Context {
        current_branch: context.branch.clone(),
        base: None,
        user_prompt: context.prompt.clone(),
        commits: String::new(),
        committed_files: String::new(),
        committed_stat: String::new(),
        committed_diff: String::new(),
        pending: commit_context.as_ref().map(Into::into),
        issues: Vec::new(),
    };
    let (generated, elapsed) = tui::timed_spinner("Generating branch workflow", || {
        generation::generate(
            &generation_context,
            Request {
                branch: true,
                commit: commit_context.is_some(),
                pull_request: false,
            },
        )
    })?;
    let branch = generated.branch.expect("generation requires branch");

    tui::step("Workflow generated", elapsed);

    tui::section("Branch");
    tui::message(&branch);

    if let Some(mut commit_context) = commit_context.take() {
        let message = generated.commit.expect("generation requires commit");
        tui::section("Commit");
        tui::message(&message);

        if tui::confirm(&format!("Create, checkout, commit, and push {}?", branch))? {
            tui::spinner("Creating branch", || git::create_branch(&branch))?;
            tui::success("Checked out", &branch);
            tui::rail();
            commit_context.branch = branch.clone();
            let commit = commit::prepare(commit_context, message, None, git::has_origin_remote());
            commit::finish(&commit)?;
        } else {
            tui::aborted();
        }
    } else if tui::confirm(&format!("Create, checkout, and push {}?", branch))? {
        tui::spinner("Creating branch", || git::create_branch(&branch))?;
        tui::success("Checked out", &branch);
        tui::rail();
        tui::spinner("Pushing branch", || git::push_branch(&branch))?;
        tui::success("Pushed to", &format!("origin/{}", branch));
    } else {
        tui::aborted();
    }

    Ok(())
}
