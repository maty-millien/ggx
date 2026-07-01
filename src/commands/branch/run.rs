use crate::commands::branch::{context::Context, git, name, prompt};
use crate::{ai, tui};
use std::time::Instant;

pub fn run(input_prompt: Option<String>) -> anyhow::Result<()> {
    let started = Instant::now();
    let context = Context::collect(input_prompt)?;

    tui::step("Analysis complete", started.elapsed());

    let branch = generate_unique_branch(&context)?;

    tui::section("Branch");
    tui::message(&branch);

    if tui::confirm(&format!("Create and checkout {}?", branch))? {
        tui::spinner("Creating branch", || git::create(&branch))?;
        tui::success("Checked out", &branch);
    } else {
        tui::warning("Aborted");
    }

    Ok(())
}

fn generate_unique_branch(context: &Context) -> anyhow::Result<String> {
    let (branch, elapsed) =
        tui::timed_spinner("Generating branch name", || generate_branch(context, None))?;

    tui::step("Branch generated", elapsed);

    if !git::branch_exists(&branch)? {
        return Ok(branch);
    }

    tui::warning(&format!(
        "Branch {} already exists; generating another",
        branch
    ));

    let forbidden = branch;
    let (branch, elapsed) = tui::timed_spinner("Generating branch name", || {
        generate_branch(context, Some(&forbidden))
    })?;

    tui::step("Branch generated", elapsed);

    if git::branch_exists(&branch)? {
        anyhow::bail!("Generated branch '{}' already exists.", branch);
    }

    Ok(branch)
}

fn generate_branch(context: &Context, forbidden: Option<&str>) -> anyhow::Result<String> {
    let output = ai::generate(&prompt::render(context, forbidden))?;

    name::normalize(&output)
}
