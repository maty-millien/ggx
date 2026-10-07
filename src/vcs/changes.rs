use crate::tui::{ChangeRow, ChangeStatus};

struct ParsedChange {
    path: String,
    additions: String,
    deletions: String,
}

pub fn from_files_and_numstat(files: &str, numstat: &str) -> Vec<ChangeRow> {
    let stats = numstat
        .lines()
        .filter_map(parse_numstat)
        .collect::<Vec<_>>();

    files
        .lines()
        .map(|line| {
            let (status, path) = parse_file_line(line);
            let stat = stats.iter().find(|stat| stat.path == path);

            ChangeRow {
                status: change_status(&status),
                path,
                additions: stat.map(|stat| stat.additions.clone()),
                deletions: stat.map(|stat| stat.deletions.clone()),
            }
        })
        .collect()
}

fn parse_file_line(line: &str) -> (String, String) {
    let mut parts = line.split('\t');
    (
        parts.next().unwrap_or_default().to_string(),
        parts.next_back().unwrap_or_default().to_string(),
    )
}

fn parse_numstat(line: &str) -> Option<ParsedChange> {
    let mut parts = line.split('\t');
    let additions = parts.next()?.to_string();
    let deletions = parts.next()?.to_string();
    let path = parts.next_back()?.to_string();

    Some(ParsedChange {
        path,
        additions,
        deletions,
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
