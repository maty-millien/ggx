use crate::ai;
use crate::commands::commit::context::Context;
use crate::git;
use crate::ui::{self, ChangeRow, ChangeStatus};
use std::time::Instant;

pub fn run() -> anyhow::Result<()> {
    let started = Instant::now();
    let context = Context::collect()?;
    let upstream = git::upstream_branch()?;

    ui::step("Analysis complete", started.elapsed());
    ui::section("Changes");
    ui::change_rows(&changes(&context));

    let (message, elapsed) = ui::timed_spinner("Generating commit message", || {
        ai::generate(&context.render_prompt())
    })?;

    ui::step("Message generated", elapsed);
    ui::message(&message);
    if confirm_commit(&context.branch)? {
        commit_and_push(&context, &message, upstream.as_deref())?;
    } else {
        ui::warning("Aborted");
    }

    Ok(())
}

fn commit_and_push(context: &Context, message: &str, upstream: Option<&str>) -> anyhow::Result<()> {
    if context.stage_before_commit {
        ui::spinner("Staging changes", git::stage_all)?;
    }

    ui::spinner("Creating commit", || git::commit(message))?;
    ui::success("Committed to", &context.branch);

    if let Some(upstream) = upstream {
        ui::spinner("Pushing commit", git::push)?;
        ui::success("Pushed to", upstream);
    }

    Ok(())
}

fn confirm_commit(branch: &str) -> anyhow::Result<bool> {
    ui::confirm(&format!("Commit to {}?", branch))
}

struct ParsedChange {
    path: String,
    additions: Option<String>,
    deletions: Option<String>,
}

fn changes(context: &Context) -> Vec<ChangeRow> {
    let stats = context
        .numstat
        .lines()
        .filter_map(parse_numstat)
        .collect::<Vec<_>>();

    context
        .files
        .lines()
        .map(|line| {
            let (status, path) = parse_file_line(line);
            let stat = stats.iter().find(|stat| stat.path == path);

            ChangeRow {
                status: change_status(&status),
                path,
                additions: stat.and_then(|stat| stat.additions.clone()),
                deletions: stat.and_then(|stat| stat.deletions.clone()),
            }
        })
        .collect()
}

fn parse_file_line(line: &str) -> (String, String) {
    if line.contains('\t') {
        let mut parts = line.split('\t');
        return (
            parts.next().unwrap_or_default().to_string(),
            parts.last().unwrap_or_default().to_string(),
        );
    }

    let status = line.get(..2).unwrap_or_default().trim().to_string();
    let path = line.get(3..).unwrap_or_default().to_string();

    (status, path)
}

fn parse_numstat(line: &str) -> Option<ParsedChange> {
    let mut parts = line.split('\t');
    let additions = parts.next()?.to_string();
    let deletions = parts.next()?.to_string();
    let path = parts.last()?.to_string();

    Some(ParsedChange {
        path,
        additions: Some(additions),
        deletions: Some(deletions),
    })
}

fn change_status(status: &str) -> ChangeStatus {
    match status.chars().next() {
        Some('A') | Some('?') => ChangeStatus::Added,
        Some('D') => ChangeStatus::Deleted,
        Some('R') => ChangeStatus::Renamed,
        Some('M') => ChangeStatus::Modified,
        _ => ChangeStatus::Unknown,
    }
}
