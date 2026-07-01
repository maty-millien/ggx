use crate::commands::commit::context::Context;
use crate::tui;

pub fn upstream() -> Option<String> {
    crate::git::run(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .ok()
        .map(|output| output.trim().to_string())
}

pub fn commit_and_push(
    context: &Context,
    message: &str,
    upstream: Option<&str>,
) -> anyhow::Result<()> {
    if context.stage_before_commit {
        tui::spinner("Staging changes", || crate::git::run(&["add", "--all"]))?;
    }

    tui::spinner("Creating commit", || {
        crate::git::run(&["commit", "-m", message])
    })?;
    tui::success("Committed to", &context.branch);

    if let Some(upstream) = upstream {
        tui::rail();
        tui::spinner("Pushing commit", || crate::git::run(&["push"]))?;
        tui::success("Pushed to", upstream);
    }

    Ok(())
}
