use crate::ai::Provider;
use crate::commands::commit::{self, Changes};
use crate::commands::generation::{self, Context};
use crate::{git, tui};
use anyhow::bail;
use std::time::Instant;

pub fn run(provider: Provider, prompt: Option<String>, yes: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    let prompt = prompt
        .as_deref()
        .map(str::trim)
        .filter(|prompt| !prompt.is_empty());
    let has_changes = git::has_changes()?;
    if prompt.is_none() && !has_changes {
        bail!("No staged or unstaged changes found.");
    }
    let current_branch = git::run(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    git::ensure_no_conflicts()?;
    let changes = if has_changes {
        Some(Changes::collect()?)
    } else {
        None
    };

    tui::step("Analysis complete", started.elapsed());
    if let Some(changes) = &changes {
        tui::change_rows(&commit::rows(&changes.files, &changes.numstat));
    }

    let (generated, elapsed) = tui::timed_spinner("Generating branch workflow", || {
        let context = Context {
            current_branch: &current_branch,
            new_branch: true,
            user_prompt: prompt,
            pending: changes.as_ref(),
            ..Default::default()
        };
        generation::generate(provider, &context)
    })?;
    let branch = generated.branch.expect("generation requires branch");

    tui::step("Workflow generated", elapsed);
    tui::section("Branch");
    tui::message(&branch);
    if let Some(message) = &generated.commit {
        tui::section("Commit");
        tui::message(message);
    }

    let (action, local) = if generated.commit.is_some() {
        (
            "Create, checkout, commit, and push",
            "Create, checkout, and commit",
        )
    } else {
        ("Create, checkout, and push", "Create and checkout")
    };
    let choice = if git::has_origin_remote() {
        tui::confirm_push(
            yes,
            &format!("{action} {branch}"),
            &format!("{local} {branch} without pushing"),
        )?
    } else {
        tui::confirm(yes, &format!("{action} {branch}?"))?.then_some(true)
    };
    let Some(push) = choice else {
        tui::aborted();
        return Ok(());
    };

    tui::spinner("Creating branch", || git::run(&["checkout", "-b", &branch]))?;
    tui::success("Checked out", &branch);
    match &generated.commit {
        Some(message) => {
            tui::rail();
            commit::finish(&branch, message, None, push)
        }
        None if push => {
            tui::rail();
            commit::push(&branch, None, "Pushing branch")
        }
        None => Ok(()),
    }
}
