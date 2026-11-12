use crate::ai::Provider;
use crate::commands::commit::{self, Changes};
use crate::commands::generation::{self, Context};
use crate::{git, tui};
use anyhow::bail;
use std::time::Instant;

pub fn run(provider: Provider, prompt: Option<String>, all: bool, yes: bool) -> anyhow::Result<()> {
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
        Some(Changes::collect(all)?)
    } else {
        None
    };

    tui::step("Analysis complete", started.elapsed());
    if let Some(changes) = &changes {
        tui::change_rows(&commit::rows(&changes.files, &changes.numstat));
    }

    let (action, local) = if changes.is_some() {
        (
            "Create, checkout, commit, and push",
            "Create, checkout, and commit",
        )
    } else {
        ("Create, checkout, and push", "Create and checkout")
    };
    let origin = git::has_origin_remote();
    let context = Context {
        current_branch: &current_branch,
        new_branch: true,
        user_prompt: prompt,
        pending: changes.as_ref(),
        ..Default::default()
    };
    let Some((generated, push)) = generation::review(
        provider,
        &context,
        "Generating branch workflow",
        |generated, elapsed| {
            let branch = generated
                .branch
                .as_deref()
                .expect("generation requires branch");
            tui::step("Workflow generated", elapsed);
            tui::section("Branch");
            tui::message(branch);
            if let Some(message) = &generated.commit {
                tui::section("Commit");
                tui::message(message);
            }
            let local = format!("{local} {branch} without pushing");
            tui::review(
                yes,
                &format!("{action} {branch}"),
                origin.then_some(local.as_str()),
            )
        },
    )?
    else {
        return Ok(());
    };
    let branch = generated.branch.expect("generation requires branch");

    tui::spinner("Creating branch", || git::run(&["checkout", "-b", &branch]))?;
    tui::success("Checked out", &branch);
    match &generated.commit {
        Some(message) => {
            tui::rail();
            commit::finish(
                &branch,
                message,
                None,
                push,
                changes.is_none_or(|changes| changes.all),
            )
        }
        None if push => {
            tui::rail();
            commit::push(&branch, None, "Pushing branch")
        }
        None => Ok(()),
    }
}
