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
            add_candidate(&mut candidates, &branch.name, base, starting_branch);
        }
    }

    candidates.into_iter().collect()
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

#[cfg(test)]
mod tests {
    use super::{branch_label, cleanup_candidates};
    use crate::vcs::git::LocalBranch;

    fn branch(name: &str, upstream_status: &str) -> LocalBranch {
        LocalBranch {
            name: name.to_string(),
            upstream_status: upstream_status.to_string(),
        }
    }

    #[test]
    fn branch_label_pluralizes_counts() {
        assert_eq!(branch_label(1), "branch");
        assert_eq!(branch_label(0), "branches");
        assert_eq!(branch_label(2), "branches");
    }

    #[test]
    fn cleanup_candidates_excludes_base_branch() {
        let candidates = cleanup_candidates(
            &["main".to_string(), "feature".to_string()],
            &[],
            "main",
            "topic",
        );

        assert_eq!(candidates, ["feature"]);
    }

    #[test]
    fn cleanup_candidates_excludes_starting_branch() {
        let candidates = cleanup_candidates(
            &[
                "main".to_string(),
                "topic".to_string(),
                "feature".to_string(),
            ],
            &[],
            "main",
            "topic",
        );

        assert_eq!(candidates, ["feature"]);
    }

    #[test]
    fn cleanup_candidates_includes_merged_local_branches() {
        let candidates = cleanup_candidates(
            &["feature".to_string(), "fix".to_string()],
            &[],
            "main",
            "topic",
        );

        assert_eq!(candidates, ["feature", "fix"]);
    }

    #[test]
    fn cleanup_candidates_includes_gone_branches_without_ahead_commits() {
        let candidates = cleanup_candidates(
            &[],
            &[branch("old", "[gone]"), branch("current", "[gone]")],
            "main",
            "current",
        );

        assert_eq!(candidates, ["old"]);
    }

    #[test]
    fn cleanup_candidates_skips_gone_branches_with_ahead_commits() {
        let candidates = cleanup_candidates(
            &[],
            &[
                branch("old", "[gone]"),
                branch("work", "[gone, ahead 1]"),
                branch("feature", "[ahead 2]"),
            ],
            "main",
            "topic",
        );

        assert_eq!(candidates, ["old"]);
    }
}
