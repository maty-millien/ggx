pub(super) struct BudgetedDiff {
    pub(super) value: String,
    pub(super) total_truncated: bool,
    pub(super) file_truncated: bool,
}

pub(super) fn truncate(value: String, max_chars: usize) -> (String, bool) {
    if value.chars().count() <= max_chars {
        return (value, false);
    }

    let truncated = value.chars().take(max_chars).collect();

    (truncated, true)
}

pub(super) fn budget_diff(value: String, max_chars: usize) -> BudgetedDiff {
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
    use super::{budget_diff, truncate};

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
    fn budget_diff_gives_unused_budget_to_larger_sections() {
        let small = "diff --git a/a b/a\n+a\n";
        let large = format!("diff --git a/b b/b\n{}", "+bbbbbbbbb\n".repeat(10));
        let diff = format!("{small}{large}");

        let budgeted = budget_diff(diff, 80);

        assert!(budgeted.value.starts_with(small));
        assert_eq!(budgeted.value.chars().count(), 80);
        assert!(budgeted.value.chars().count() > small.chars().count() + 40);
        assert!(budgeted.total_truncated);
        assert!(budgeted.file_truncated);
    }

    #[test]
    fn budget_diff_keeps_preamble_and_ignores_inline_diff_markers() {
        let diff = format!(
            "warning: preamble\ndiff --git a/a b/a\n+text with diff --git inside\n{}",
            "+aaaaaaaaaa\n".repeat(10)
        );

        let budgeted = budget_diff(diff, 60);

        assert!(
            budgeted
                .value
                .starts_with("warning: preamble\ndiff --git a/a b/a\n")
        );
        assert_eq!(budgeted.value.chars().count(), 60);
        assert!(budgeted.total_truncated);
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

    #[test]
    fn budget_diff_handles_zero_budget() {
        let budgeted = budget_diff("diff --git a/a b/a\n+hello\n".to_string(), 0);

        assert!(budgeted.value.is_empty());
        assert!(budgeted.total_truncated);
        assert!(budgeted.file_truncated);
    }

    #[test]
    fn budget_diff_truncates_plain_text_without_diff_sections() {
        let budgeted = budget_diff("plain text diff".to_string(), 5);

        assert_eq!(budgeted.value, "plain");
        assert!(budgeted.total_truncated);
        assert!(budgeted.file_truncated);
    }

    #[test]
    fn truncate_tracks_char_boundary() {
        let (value, truncated) = truncate("éclair".to_string(), 2);

        assert_eq!(value, "éc");
        assert!(truncated);
    }
}
