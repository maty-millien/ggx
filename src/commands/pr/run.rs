use crate::commands::pr::{context::Context, prompt, validation};
use crate::vcs::{changes, git, github};
use crate::{ai, tui};
use std::time::Instant;

pub fn run(draft: bool, closes: Vec<String>) -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_clean_worktree()?;
    let upstream = git::upstream()?;
    let context = Context::collect(closes)?;

    tui::step("Analysis complete", started.elapsed());
    tui::section("Changes");
    tui::change_rows(&changes::from_files_and_numstat(
        &context.files,
        &context.numstat,
    ));

    let (generated, elapsed) = tui::timed_spinner("Generating pull request", || {
        ai::generate(&prompt::render(&context))
    })?;
    let pull_request = validation::PullRequest::parse(&generated)?;

    tui::step("Pull request generated", elapsed);
    tui::section("Title");
    tui::message(&pull_request.title);
    tui::section("Body");
    tui::block(&pull_request.body);

    let confirm_prompt = format!("Sync {} and create PR into {}?", upstream, context.base);

    if tui::confirm(&confirm_prompt)? {
        tui::spinner("Pushing branch", git::push)?;
        tui::success("Pushed to", &upstream);
        tui::rail();
        let url = tui::spinner("Creating pull request", || {
            github::create_pr(
                &context.base,
                &context.branch,
                &pull_request.title,
                &pull_request.body,
                draft,
            )
        })?;
        tui::success("Created PR", &url);
    } else {
        tui::warning("Aborted");
    }

    Ok(())
}
