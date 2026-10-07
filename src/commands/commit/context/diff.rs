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
