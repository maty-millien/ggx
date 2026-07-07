use crate::ai;
use crate::commands::commit::{context::Context, prompt, validation};
use crate::tui;
use crate::vcs::{changes, git};
use std::time::Instant;

pub fn run() -> anyhow::Result<()> {
    let started = Instant::now();
    let prepared = prepare(
        git::current_branch_name()?,
        git::optional_upstream(),
        git::has_origin_remote(),
        started,
    )?;

    if tui::confirm(&action_prompt(&prepared))? {
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

pub(crate) fn prepare_for_new_branch(
    branch: &str,
    started: Instant,
) -> anyhow::Result<PreparedCommit> {
    prepare(branch.to_string(), None, git::has_origin_remote(), started)
}

fn prepare(
    branch: String,
    upstream: Option<String>,
    has_origin_remote: bool,
    started: Instant,
) -> anyhow::Result<PreparedCommit> {
    git::ensure_no_conflicts()?;
    tui::spinner("Staging changes", git::stage_all)?;
    let context = Context::collect_for_branch(branch)?;

    tui::step("Analysis complete", started.elapsed());
    tui::section("Changes");
    tui::change_rows(&changes::from_files_and_numstat(
        &context.files,
        &context.numstat,
    ));

    let prompt = prompt::render(&context);
    let (message, elapsed) = tui::timed_spinner("Generating commit message", || {
        generate_valid_message(&prompt)
    })?;

    tui::step("Message generated", elapsed);
    tui::message(&message);

    Ok(PreparedCommit {
        context,
        message,
        upstream,
        has_origin_remote,
    })
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

fn generate_valid_message(prompt: &str) -> anyhow::Result<String> {
    let first = ai::generate(prompt)?;
    if validation::validate(&first).is_ok() {
        return Ok(first);
    }

    let second = ai::generate(prompt)?;
    validation::validate(&second)
        .map_err(|error| anyhow::anyhow!("AI generated an invalid commit message: {}", error))?;

    Ok(second)
}

#[cfg(test)]
mod tests {
    use super::{PreparedCommit, action_prompt};
    use crate::commands::commit::context::Context;

    fn commit(upstream: Option<&str>, has_origin_remote: bool) -> PreparedCommit {
        PreparedCommit {
            context: Context {
                branch: "feat/new-flow".to_string(),
                files: "M  src/main.rs".to_string(),
                stat: "1 file changed".to_string(),
                numstat: "1\t0\tsrc/main.rs".to_string(),
                summary: String::new(),
                readme: None,
                diff: "diff --git a/src/main.rs b/src/main.rs".to_string(),
                diff_truncated: false,
                diff_file_truncated: false,
                readme_truncated: false,
            },
            message: "feat(cli): update branch flow".to_string(),
            upstream: upstream.map(str::to_string),
            has_origin_remote,
        }
    }

    #[test]
    fn action_prompt_uses_upstream_when_present() {
        assert_eq!(
            action_prompt(&commit(Some("origin/current"), true)),
            "Commit and push to origin/current?"
        );
    }

    #[test]
    fn action_prompt_uses_origin_branch_without_upstream() {
        assert_eq!(
            action_prompt(&commit(None, true)),
            "Commit and push to origin/feat/new-flow?"
        );
    }

    #[test]
    fn action_prompt_skips_push_without_origin() {
        assert_eq!(
            action_prompt(&commit(None, false)),
            "Commit to feat/new-flow?"
        );
    }
}
