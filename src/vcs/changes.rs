use crate::tui::{ChangeRow, ChangeStatus};

struct ParsedChange {
    path: String,
    additions: Option<String>,
    deletions: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::from_files_and_numstat;
    use crate::tui::ChangeStatus;

    #[test]
    fn parses_all_supported_statuses() {
        let rows = from_files_and_numstat(
            "A  added.rs\nM  modified.rs\nD  deleted.rs\nR  old.rs -> new.rs\n?? untracked.rs\nX  unknown.rs",
            "1\t0\tadded.rs\n2\t1\tmodified.rs\n0\t3\tdeleted.rs\n4\t0\told.rs -> new.rs\n5\t0\tuntracked.rs",
        );

        assert_eq!(rows[0].status, ChangeStatus::Added);
        assert_eq!(rows[1].status, ChangeStatus::Modified);
        assert_eq!(rows[2].status, ChangeStatus::Deleted);
        assert_eq!(rows[3].status, ChangeStatus::Renamed);
        assert_eq!(rows[4].status, ChangeStatus::Added);
        assert_eq!(rows[5].status, ChangeStatus::Unknown);
        assert_eq!(rows[0].additions.as_deref(), Some("1"));
        assert_eq!(rows[5].additions, None);
    }

    #[test]
    fn keeps_paths_with_spaces() {
        let rows = from_files_and_numstat("M  docs/my file.md", "10\t2\tdocs/my file.md");

        assert_eq!(rows[0].path, "docs/my file.md");
        assert_eq!(rows[0].additions.as_deref(), Some("10"));
        assert_eq!(rows[0].deletions.as_deref(), Some("2"));
    }

    #[test]
    fn parses_tab_rename_format_using_new_path() {
        let rows = from_files_and_numstat("R100\told name.rs\tnew name.rs", "1\t1\tnew name.rs");

        assert_eq!(rows[0].status, ChangeStatus::Renamed);
        assert_eq!(rows[0].path, "new name.rs");
        assert_eq!(rows[0].additions.as_deref(), Some("1"));
    }

    #[test]
    fn preserves_binary_numstat_markers() {
        let rows = from_files_and_numstat("M  image.png", "-\t-\timage.png");

        assert_eq!(rows[0].additions.as_deref(), Some("-"));
        assert_eq!(rows[0].deletions.as_deref(), Some("-"));
    }

    #[test]
    fn omits_stats_when_no_numstat_matches() {
        let rows = from_files_and_numstat("M  src/lib.rs", "1\t1\tother.rs");

        assert_eq!(rows[0].path, "src/lib.rs");
        assert_eq!(rows[0].additions, None);
        assert_eq!(rows[0].deletions, None);
    }
}
