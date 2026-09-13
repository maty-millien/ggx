use super::LocalBranch;

pub(super) fn failure_message(args: &[&str], stderr: &[u8]) -> String {
    let command = format!("git {} failed", args.join(" "));
    let detail = String::from_utf8_lossy(stderr);
    let detail = detail.trim();

    if detail.is_empty() {
        command
    } else {
        format!("{command}: {detail}")
    }
}

pub(super) fn has_output(output: &str) -> bool {
    !output.trim().is_empty()
}

pub(super) fn parse_branch_names(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

pub(super) fn parse_local_branches(output: &str) -> Vec<LocalBranch> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            let (name, upstream_status) = line.split_once('\t').unwrap_or((line, ""));

            Some(LocalBranch {
                name: name.trim().to_string(),
                upstream_status: upstream_status.trim().to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{failure_message, has_output, parse_branch_names, parse_local_branches};

    #[test]
    fn failure_message_includes_git_stderr() {
        assert_eq!(
            failure_message(
                &["ls-files", "--unmerged"],
                b"fatal: not a git repository\n"
            ),
            "git ls-files --unmerged failed: fatal: not a git repository"
        );
    }

    #[test]
    fn failure_message_falls_back_when_stderr_is_empty() {
        assert_eq!(
            failure_message(&["status", "--porcelain"], b""),
            "git status --porcelain failed"
        );
    }

    #[test]
    fn has_output_rejects_empty_or_whitespace() {
        assert!(!has_output(""));
        assert!(!has_output(" \n\t "));
    }

    #[test]
    fn has_output_accepts_non_whitespace() {
        assert!(has_output("main\n"));
    }

    #[test]
    fn parse_branch_names_omits_empty_lines() {
        assert_eq!(
            parse_branch_names("main\n\nfeature\n"),
            vec!["main".to_string(), "feature".to_string()]
        );
    }

    #[test]
    fn parse_local_branches_reads_optional_upstream_status() {
        let branches = parse_local_branches("main\t\nfeature\t[gone]\n\nlegacy\n");

        assert_eq!(branches.len(), 3);
        assert_eq!(branches[0].name, "main");
        assert_eq!(branches[0].upstream_status, "");
        assert_eq!(branches[1].name, "feature");
        assert_eq!(branches[1].upstream_status, "[gone]");
        assert_eq!(branches[2].name, "legacy");
        assert_eq!(branches[2].upstream_status, "");
    }
}
