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
