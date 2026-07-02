use crate::git;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_DIFF_CHARS: usize = 16_000;
const MAX_README_CHARS: usize = 8_000;
const README_DIRS: &[&str] = &[".", "docs"];
const README_NAMES: &[&str] = &["readme", "readme.md", "readme.markdown", "readme.txt"];

pub struct Context {
    pub branch: String,
    pub files: String,
    pub stat: String,
    pub numstat: String,
    pub summary: String,
    pub readme: Option<String>,
    pub diff: String,
    pub diff_truncated: bool,
    pub diff_file_truncated: bool,
    pub readme_truncated: bool,
}

impl Context {
    pub fn collect() -> anyhow::Result<Self> {
        let files = git::run(&["diff", "--staged", "--name-status"])?
            .trim()
            .to_string();
        let stat = git::run(&["diff", "--staged", "--stat"])?
            .trim()
            .to_string();
        let numstat = git::run(&["diff", "--staged", "--numstat"])?
            .trim()
            .to_string();
        let summary = git::run(&["diff", "--staged", "--summary"])?
            .trim()
            .to_string();
        let diff = git::run(&["diff", "--staged", "--unified=3"])?
            .trim()
            .to_string();

        if files.is_empty() {
            anyhow::bail!("No changes found.");
        }

        let diff = budget_diff(diff, MAX_DIFF_CHARS);
        let repo_root = git::run(&["rev-parse", "--show-toplevel"])?
            .trim()
            .to_string();
        let readme = read_readme(Path::new(&repo_root))?;
        let (readme, readme_truncated) = match readme {
            Some(readme) => {
                let (readme, truncated) = truncate(readme, MAX_README_CHARS);
                (Some(readme), truncated)
            }
            None => (None, false),
        };

        Ok(Self {
            branch: git::current_branch_name()?,
            files,
            stat,
            numstat,
            summary,
            readme,
            diff: diff.value,
            diff_truncated: diff.total_truncated,
            diff_file_truncated: diff.file_truncated,
            readme_truncated,
        })
    }
}

struct BudgetedDiff {
    value: String,
    total_truncated: bool,
    file_truncated: bool,
}

fn read_readme(root: &Path) -> anyhow::Result<Option<String>> {
    let Some(path) = find_readme(root) else {
        return Ok(None);
    };

    Ok(Some(fs::read_to_string(path)?))
}

fn find_readme(root: &Path) -> Option<PathBuf> {
    README_DIRS
        .iter()
        .filter_map(|dir| readme_in_dir(&root.join(dir)))
        .next()
}

fn readme_in_dir(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    README_NAMES.iter().find_map(|readme_name| {
        entries.iter().find_map(|entry| {
            let file_name = entry.file_name();
            let file_name = file_name.to_str()?;
            if file_name.eq_ignore_ascii_case(readme_name) {
                Some(entry.path())
            } else {
                None
            }
        })
    })
}

fn truncate(value: String, max_chars: usize) -> (String, bool) {
    if value.chars().count() <= max_chars {
        return (value, false);
    }

    let truncated = value.chars().take(max_chars).collect();

    (truncated, true)
}

fn budget_diff(value: String, max_chars: usize) -> BudgetedDiff {
    if value.chars().count() <= max_chars {
        return BudgetedDiff {
            value,
            total_truncated: false,
            file_truncated: false,
        };
    }

    if max_chars == 0 {
        return BudgetedDiff {
            value: String::new(),
            total_truncated: true,
            file_truncated: true,
        };
    }

    let sections = split_diff_sections(&value);
    let section_count = sections.len();
    let mut allocations = vec![0; section_count];

    if section_count == 0 {
        return BudgetedDiff {
            value: take_chars(&value, max_chars),
            total_truncated: true,
            file_truncated: true,
        };
    }

    let base_budget = (max_chars / section_count).max(1);
    let mut remaining = max_chars;

    for (index, section) in sections.iter().enumerate() {
        let remaining_sections = section_count - index;
        let section_budget = base_budget.min(remaining / remaining_sections).max(1);
        let allocated = section.chars().count().min(section_budget).min(remaining);
        allocations[index] = allocated;
        remaining -= allocated;
    }

    while remaining > 0 {
        let mut spent = false;
        for (index, section) in sections.iter().enumerate() {
            let section_len = section.chars().count();
            if allocations[index] < section_len {
                allocations[index] += 1;
                remaining -= 1;
                spent = true;
                if remaining == 0 {
                    break;
                }
            }
        }

        if !spent {
            break;
        }
    }

    let file_truncated = sections
        .iter()
        .zip(allocations.iter())
        .any(|(section, allocation)| section.chars().count() > *allocation);
    let value = sections
        .iter()
        .zip(allocations.iter())
        .map(|(section, allocation)| take_chars(section, *allocation))
        .collect::<String>();

    BudgetedDiff {
        value,
        total_truncated: true,
        file_truncated,
    }
}

fn split_diff_sections(value: &str) -> Vec<&str> {
    let mut starts = value
        .match_indices("diff --git ")
        .filter_map(|(index, _)| {
            if index == 0 || value[..index].ends_with('\n') {
                Some(index)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if starts.is_empty() {
        return vec![value];
    }

    if starts[0] != 0 {
        starts.insert(0, 0);
    }

    starts
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let end = starts.get(index + 1).copied().unwrap_or(value.len());
            &value[*start..end]
        })
        .collect()
}

fn take_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::budget_diff;

    #[test]
    fn budget_diff_keeps_single_small_diff() {
        let diff = String::from("diff --git a/a b/a\n+hello\n");

        let budgeted = budget_diff(diff.clone(), 100);

        assert_eq!(budgeted.value, diff);
        assert!(!budgeted.total_truncated);
        assert!(!budgeted.file_truncated);
    }

    #[test]
    fn budget_diff_represents_multiple_files_under_tight_budget() {
        let diff = [
            "diff --git a/a b/a\n",
            "@@ -1 +1 @@\n",
            "+aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
            "diff --git a/b b/b\n",
            "@@ -1 +1 @@\n",
            "+bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n",
        ]
        .concat();

        let budgeted = budget_diff(diff, 80);

        assert!(budgeted.value.contains("diff --git a/a b/a"));
        assert!(budgeted.value.contains("diff --git a/b b/b"));
        assert!(budgeted.total_truncated);
        assert!(budgeted.file_truncated);
        assert!(budgeted.value.chars().count() <= 80);
    }

    #[test]
    fn budget_diff_truncates_large_single_file_diff() {
        let diff = format!("diff --git a/a b/a\n{}", "+hello\n".repeat(100));

        let budgeted = budget_diff(diff, 60);

        assert!(budgeted.value.starts_with("diff --git a/a b/a"));
        assert!(budgeted.total_truncated);
        assert!(budgeted.file_truncated);
        assert!(budgeted.value.chars().count() <= 60);
    }

    #[test]
    fn budget_diff_keeps_empty_diff_empty() {
        let budgeted = budget_diff(String::new(), 60);

        assert!(budgeted.value.is_empty());
        assert!(!budgeted.total_truncated);
        assert!(!budgeted.file_truncated);
    }
}
