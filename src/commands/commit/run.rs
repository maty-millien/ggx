use crate::ai;
use crate::commands::commit::{changes, context::Context, prompt};
use crate::{git, tui};
use std::time::Instant;

pub fn run() -> anyhow::Result<()> {
    let started = Instant::now();
    let context = Context::collect()?;
    let upstream = git::optional_upstream();

    tui::step("Analysis complete", started.elapsed());
    tui::section("Changes");
    tui::change_rows(&changes::from_context(&context));

    let (message, elapsed) = tui::timed_spinner("Generating commit message", || {
        ai::generate(&prompt::render(&context))
    })?;

    tui::step("Message generated", elapsed);
    tui::message(&message);
    let prompt = match upstream.as_deref() {
        Some(upstream) => format!("Commit and push to {}?", upstream),
        None => format!("Commit to {}?", context.branch),
    };

    if tui::confirm(&prompt)? {
        commit_and_push(&context, &message, upstream.as_deref())?;
    } else {
        tui::warning("Aborted");
    }

    Ok(())
}

fn commit_and_push(context: &Context, message: &str, upstream: Option<&str>) -> anyhow::Result<()> {
    if context.stage_before_commit {
        tui::spinner("Staging changes", git::stage_all)?;
    }

    tui::spinner("Creating commit", || git::commit(message))?;
    tui::success("Committed to", &context.branch);

    if let Some(upstream) = upstream {
        tui::rail();
        tui::spinner("Pushing commit", git::push)?;
        tui::success("Pushed to", upstream);
    }

    Ok(())
}
