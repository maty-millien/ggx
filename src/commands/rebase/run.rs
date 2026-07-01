use crate::commands::merge_common as git;
use crate::tui;
use std::time::Instant;

pub fn run(target: Option<String>, keep_branch: bool, admin: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_clean_worktree()?;
    let branch = git::current_branch()?;
    let target = target
        .map(|target| target.trim().to_string())
        .filter(|target| !target.is_empty())
        .map(Ok)
        .unwrap_or_else(git::default_base)?;

    git::ensure_rewrite_allowed(&branch, &target, admin)?;
    git::fetch()?;
    let target_ref = git::branch_ref(&target)?;

    if branch == target {
        anyhow::bail!("Cannot rebase '{}' onto itself.", branch);
    }

    tui::step("Analysis complete", started.elapsed());
    tui::section("Rebase");
    tui::block(&format!("{} onto {}", branch, target));

    if !tui::confirm(&format!("Rebase {} onto {}?", branch, target))? {
        tui::warning("Aborted");
        return Ok(());
    }

    if let Err(error) = tui::spinner("Rebasing branch", || {
        crate::git::run(&["rebase", &target_ref])
    }) {
        tui::warning("Rebase stopped with conflicts");
        tui::block(
            "Resolve conflicts, then run git rebase --continue. To cancel, run git rebase --abort.",
        );
        return Err(error);
    }

    tui::success("Rebased", &branch);

    if let Some(upstream) = git::upstream()
        && tui::confirm(&format!("Force push {} with lease?", upstream))?
    {
        tui::spinner("Force pushing", git::force_push_with_lease)?;
        tui::success("Pushed to", &upstream);
    }

    if keep_branch {
        tui::warning("Keeping branch");
    }

    Ok(())
}
