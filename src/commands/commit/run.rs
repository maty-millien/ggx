use crate::ai;
use crate::commands::commit::{changes, context::Context, message, prompt};
use crate::{git, tui};
use std::time::Instant;

pub fn run() -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_no_conflicts()?;
    tui::spinner("Staging changes", git::stage_all)?;
    let context = Context::collect()?;
    let upstream = git::optional_upstream();
    let has_origin_remote = git::has_origin_remote();

    tui::step("Analysis complete", started.elapsed());
    tui::section("Changes");
    tui::change_rows(&changes::from_context(&context));

    let prompt = prompt::render(&context);
    let (message, elapsed) = tui::timed_spinner("Generating commit message", || {
        generate_valid_message(&prompt)
    })?;

    tui::step("Message generated", elapsed);
    tui::message(&message);
    let prompt = match upstream.as_deref() {
        Some(upstream) => format!("Commit and push to {}?", upstream),
        None if has_origin_remote => format!("Commit and push to origin/{}?", context.branch),
        None => format!("Commit to {}?", context.branch),
    };

    if tui::confirm(&prompt)? {
        commit_and_push(&context, &message, upstream.as_deref(), has_origin_remote)?;
    } else {
        tui::warning("Aborted");
    }

    Ok(())
}

fn commit_and_push(
    context: &Context,
    message: &str,
    upstream: Option<&str>,
    has_origin_remote: bool,
) -> anyhow::Result<()> {
    tui::spinner("Creating commit", || git::commit(message))?;
    tui::success("Committed to", &context.branch);

    if let Some(upstream) = upstream {
        tui::rail();
        tui::spinner("Pushing commit", git::push)?;
        tui::success("Pushed to", upstream);
    } else if has_origin_remote {
        let destination = format!("origin/{}", context.branch);
        tui::rail();
        tui::spinner("Pushing commit", || git::push_branch(&context.branch))?;
        tui::success("Pushed to", &destination);
    }

    Ok(())
}

fn generate_valid_message(prompt: &str) -> anyhow::Result<String> {
    let first = ai::generate(prompt)?;
    if message::validate(&first).is_ok() {
        return Ok(first);
    }

    let second = ai::generate(prompt)?;
    message::validate(&second)
        .map_err(|error| anyhow::anyhow!("AI generated an invalid commit message: {}", error))?;

    Ok(second)
}
