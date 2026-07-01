use crate::commands::{merge_common as git, squash::prompt};
use crate::{ai, tui};
use std::time::Instant;

const MAX_DIFF_CHARS: usize = 16_000;

pub fn run(keep_branch: bool, admin: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    let branch = git::current_branch()?;
    let base = git::default_base()?;
    git::ensure_rewrite_allowed(&branch, &base, admin)?;
    git::fetch()?;
    let base_ref = git::branch_ref(&base)?;

    if branch == base {
        anyhow::bail!("Cannot squash the base branch '{}'.", base);
    }

    let commits = crate::git::run(&["log", "--oneline", &format!("{}..HEAD", base_ref)])?
        .trim()
        .to_string();
    if commits.is_empty() {
        anyhow::bail!("No commits found between {} and {}.", base, branch);
    }

    let diff = crate::git::run(&["diff", "--unified=3", &format!("{}...HEAD", base_ref)])?
        .trim()
        .chars()
        .take(MAX_DIFF_CHARS)
        .collect::<String>();

    tui::step("Analysis complete", started.elapsed());
    tui::section("Commits");
    tui::block(&commits);

    let (message, elapsed) = tui::timed_spinner("Generating squash commit", || {
        ai::generate(&prompt::render(&branch, &base, &commits, &diff))
    })?;

    tui::step("Message generated", elapsed);
    tui::message(&message);

    if !tui::confirm(&format!("Squash {} onto {}?", branch, base))? {
        tui::warning("Aborted");
        return Ok(());
    }

    tui::spinner("Squashing commits", || {
        crate::git::run(&["reset", "--soft", &base_ref])?;
        crate::git::run(&["commit", "-m", &message])
    })?;
    tui::success("Squashed", &branch);

    if let Some(upstream) = git::upstream() {
        if !admin {
            anyhow::bail!(
                "Refusing to force-push {} without --admin. Re-run with --admin to unlock this.",
                upstream
            );
        }
        if tui::confirm(&format!("Force push {} with lease?", upstream))? {
            tui::spinner("Force pushing", git::force_push_with_lease)?;
            tui::success("Pushed to", &upstream);
        }
    }

    if keep_branch {
        tui::warning("Keeping branch");
    }

    Ok(())
}
