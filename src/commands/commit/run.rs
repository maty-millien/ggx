use crate::ai::{self, Provider};
use crate::commands::commit::context::Context;
use crate::commands::generation::{self, Request};
use crate::tui;
use crate::vcs::{changes, git};
use std::time::Instant;

pub fn run(provider: Provider, yes: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_no_conflicts()?;
    let context = Context::collect_for_branch(git::current_branch_name()?)?;

    tui::step("Analysis complete", started.elapsed());
    show_changes(&context);

    let generation_context = generation::Context {
        current_branch: context.branch.clone(),
        base: None,
        user_prompt: None,
        commits: String::new(),
        committed_files: String::new(),
        committed_stat: String::new(),
        committed_diff: String::new(),
        pending: Some((&context).into()),
        issues: Vec::new(),
    };
    let (generated, elapsed) = tui::timed_spinner("Generating commit message", || {
        generation::generate(
            &generation_context,
            Request {
                branch: false,
                commit: true,
                pull_request: false,
            },
            |prompt| ai::generate(provider, prompt),
            git::branch_exists,
        )
    })?;
    let message = generated.commit.expect("generation requires commit");

    tui::step("Message generated", elapsed);
    tui::message(&message);

    let prepared = prepare(
        context,
        message,
        git::optional_upstream(),
        git::has_origin_remote(),
    );

    if tui::confirm(yes, &action_prompt(&prepared))? {
        finish(&prepared)?;
    } else {
        tui::aborted();
    }

    Ok(())
}

pub(crate) struct PreparedCommit {
    context: Context,
    message: String,
    upstream: Option<String>,
    has_origin_remote: bool,
}

pub(crate) fn prepare(
    context: Context,
    message: String,
    upstream: Option<String>,
    has_origin_remote: bool,
) -> PreparedCommit {
    PreparedCommit {
        context,
        message,
        upstream,
        has_origin_remote,
    }
}

pub(crate) fn show_changes(context: &Context) {
    tui::section("Changes");
    tui::change_rows(&changes::from_files_and_numstat(
        &context.files,
        &context.numstat,
    ));
}

pub(crate) fn action_prompt(commit: &PreparedCommit) -> String {
    match commit.upstream.as_deref() {
        Some(upstream) => format!("Commit and push to {}?", upstream),
        None if commit.has_origin_remote => {
            format!("Commit and push to origin/{}?", commit.context.branch)
        }
        None => format!("Commit to {}?", commit.context.branch),
    }
}

pub(crate) fn finish(commit: &PreparedCommit) -> anyhow::Result<()> {
    tui::spinner("Staging changes", git::stage_all)?;
    tui::spinner("Creating commit", || git::commit(&commit.message))?;
    tui::success("Committed to", &commit.context.branch);

    if let Some(upstream) = commit.upstream.as_deref() {
        tui::rail();
        tui::spinner("Pushing commit", git::push)?;
        tui::success("Pushed to", upstream);
    } else if commit.has_origin_remote {
        let destination = format!("origin/{}", commit.context.branch);
        tui::rail();
        tui::spinner("Pushing commit", || {
            git::push_branch(&commit.context.branch)
        })?;
        tui::success("Pushed to", &destination);
    }

    Ok(())
}
