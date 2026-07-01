use crate::ai;
use crate::commands::commit::context::Context;
use crate::git;
use crate::tui::{self, ChangeRow, ChangeStatus};
use std::time::Instant;

pub fn run() -> anyhow::Result<()> {
    let started = Instant::now();
    let context = Context::collect()?;
    let upstream = git::upstream_branch()?;

    tui::step("Analysis complete", started.elapsed());
    tui::section("Changes");
    tui::change_rows(&changes(&context));

    let (message, elapsed) = tui::timed_spinner("Generating commit message", || {
        ai::generate(&context.render_prompt())
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
            parts.next_back().unwrap_or_default().to_string(),
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
    let path = parts.next_back()?.to_string();

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
