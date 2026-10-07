use crate::vcs::git::LocalBranch;
use std::collections::BTreeSet;

pub(super) fn branch_label(count: usize) -> &'static str {
    if count == 1 { "branch" } else { "branches" }
}

pub(super) fn cleanup_candidates(
    merged_branches: &[String],
    local_branches: &[LocalBranch],
    base: &str,
    starting_branch: &str,
) -> Vec<String> {
    let mut candidates = BTreeSet::new();

    for branch in merged_branches {
        add_candidate(&mut candidates, branch, base, starting_branch);
    }

    for branch in local_branches {
        if is_gone_without_ahead(&branch.upstream_status) {
            add_gone_candidate(&mut candidates, &branch.name, base);
        }
    }

    candidates.into_iter().collect()
}

fn add_gone_candidate(candidates: &mut BTreeSet<String>, branch: &str, base: &str) {
    if branch != base {
        candidates.insert(branch.to_string());
    }
}

fn add_candidate(
    candidates: &mut BTreeSet<String>,
    branch: &str,
    base: &str,
    starting_branch: &str,
) {
    if branch != base && branch != starting_branch {
        candidates.insert(branch.to_string());
    }
}

fn is_gone_without_ahead(status: &str) -> bool {
    status.contains("gone") && !status.contains("ahead")
}
