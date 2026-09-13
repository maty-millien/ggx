use console::style;

pub(super) fn rail_text() -> console::StyledObject<&'static str> {
    style("│").dim()
}

pub(super) fn change_status(status: ChangeStatus) -> console::StyledObject<&'static str> {
    match status {
        ChangeStatus::Added => style("A").green(),
        ChangeStatus::Modified => style("M").yellow(),
        ChangeStatus::Deleted => style("D").red(),
        ChangeStatus::Renamed => style("R").cyan(),
        ChangeStatus::Unknown => style("?").dim(),
    }
}

pub(super) fn path(path: &str) -> String {
    if path.ends_with('/') {
        return style(path).bold().to_string();
    }

    let Some((dir, file)) = path.rsplit_once('/') else {
        return style(path).bold().to_string();
    };

    format!("{}/{}", style(dir).dim(), style(file).bold())
}

pub(super) fn addition(value: Option<&str>) -> String {
    match value {
        Some("-") | None => String::new(),
        Some("0") => String::new(),
        Some(value) => format!(" {}", style(format!("+{}", value)).green()),
    }
}

pub(super) fn deletion(value: Option<&str>) -> String {
    match value {
        Some("-") | None => String::new(),
        Some("0") => String::new(),
        Some(value) => format!(" {}", style(format!("-{}", value)).red()),
    }
}

pub(super) fn commit_message(message: &str) -> String {
    let Some((kind, rest)) = message.split_once(':') else {
        return style(message).green().bold().to_string();
    };

    format!("{}:{}", commit_type(kind), style(rest).white())
}

pub(super) fn commit_type(kind: &str) -> String {
    let Some((name, scope)) = kind.split_once('(') else {
        return style(kind).green().bold().to_string();
    };

    format!("{}({}", style(name).green().bold(), style(scope).cyan())
}

pub(super) fn wrap_line(line: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let indent = line
        .chars()
        .take_while(|character| character.is_whitespace())
        .collect::<String>();
    let width = width.saturating_sub(indent.chars().count()).max(1);

    for word in line.split_whitespace() {
        if word.chars().count() > width {
            if !current.is_empty() {
                lines.push(format!("{}{}", indent, current));
                current = String::new();
            }
            lines.extend(
                split_word(word, width)
                    .into_iter()
                    .map(|word| format!("{}{}", indent, word)),
            );
            continue;
        }

        let separator = if current.is_empty() { 0 } else { 1 };
        if current.chars().count() + separator + word.chars().count() > width && !current.is_empty()
        {
            lines.push(format!("{}{}", indent, current));
            current = String::new();
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if current.is_empty() && lines.is_empty() {
        lines.push(line.to_string());
    } else if !current.is_empty() {
        lines.push(format!("{}{}", indent, current));
    }

    lines
}

pub(super) fn split_word(word: &str, width: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();

    for character in word.chars() {
        if current.chars().count() >= width {
            parts.push(current);
            current = String::new();
        }
        current.push(character);
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

pub(super) fn visual_rows(line: &str, columns: usize) -> usize {
    let width = console::measure_text_width(line);
    width.div_ceil(columns.max(1)).max(1)
}

pub(super) fn selected_line(label: &str) -> String {
    format!("{} {}", rail_text(), style(label).dim())
}

pub(super) fn select_line(label: &str, selected: bool) -> String {
    if selected {
        return format!("  {} {}", style("●").green(), style(label).bold());
    }

    format!("  {} {}", style("○").dim(), style(label).dim())
}

pub(super) fn confirm_label(prompt: &str) -> &str {
    prompt.trim().trim_end_matches('?')
}

pub(super) fn cancel_choice<T>(choices: &[Choice<'_, T>]) -> Option<usize> {
    choices
        .iter()
        .position(|choice| choice.label.eq_ignore_ascii_case("cancel"))
}

pub(super) fn digit_key(character: char) -> SelectKey {
    character
        .to_digit(10)
        .and_then(|digit| usize::try_from(digit).ok())
        .and_then(|digit| digit.checked_sub(1))
        .map_or(SelectKey::Ignore, SelectKey::Index)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Unknown,
}

pub struct ChangeRow {
    pub status: ChangeStatus,
    pub path: String,
    pub additions: Option<String>,
    pub deletions: Option<String>,
}

pub struct Choice<'a, T> {
    pub(super) label: &'a str,
    pub(super) value: T,
}

impl<'a, T> Choice<'a, T> {
    pub fn new(label: &'a str, value: T) -> Self {
        Self { label, value }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SelectKey {
    Confirm,
    Cancel,
    Next,
    Previous,
    Index(usize),
    Ignore,
}

#[cfg(test)]
mod tests {
    use super::{
        ChangeStatus, Choice, addition, cancel_choice, change_status, commit_message,
        confirm_label, deletion, digit_key, path, select_line, selected_line, visual_rows,
        wrap_line,
    };

    fn disable_colors() {
        console::set_colors_enabled(false);
    }

    #[test]
    fn change_status_maps_each_status_to_a_marker() {
        disable_colors();

        assert_eq!(change_status(ChangeStatus::Added).to_string(), "A");
        assert_eq!(change_status(ChangeStatus::Modified).to_string(), "M");
        assert_eq!(change_status(ChangeStatus::Deleted).to_string(), "D");
        assert_eq!(change_status(ChangeStatus::Renamed).to_string(), "R");
        assert_eq!(change_status(ChangeStatus::Unknown).to_string(), "?");
    }

    #[test]
    fn path_formats_plain_file_and_nested_path() {
        disable_colors();

        assert_eq!(path("README.md"), "README.md");
        assert_eq!(path("src/main.rs"), "src/main.rs");
        assert_eq!(path("src/"), "src/");
    }

    #[test]
    fn addition_suppresses_empty_zero_and_binary_values() {
        disable_colors();

        assert_eq!(addition(None), "");
        assert_eq!(addition(Some("0")), "");
        assert_eq!(addition(Some("-")), "");
        assert_eq!(addition(Some("3")), " +3");
    }

    #[test]
    fn deletion_suppresses_empty_zero_and_binary_values() {
        disable_colors();

        assert_eq!(deletion(None), "");
        assert_eq!(deletion(Some("0")), "");
        assert_eq!(deletion(Some("-")), "");
        assert_eq!(deletion(Some("2")), " -2");
    }

    #[test]
    fn commit_message_formats_conventional_type_and_falls_back() {
        disable_colors();

        assert_eq!(
            commit_message("feat(cli): add command"),
            "feat(cli): add command"
        );
        assert_eq!(commit_message("feat: add command"), "feat: add command");
        assert_eq!(commit_message("plain message"), "plain message");
    }

    #[test]
    fn wrap_line_keeps_short_and_empty_lines() {
        assert_eq!(wrap_line("one two", 10), vec!["one two"]);
        assert_eq!(wrap_line("", 10), vec![""]);
        assert_eq!(wrap_line("   ", 10), vec!["   "]);
    }

    #[test]
    fn wrap_line_breaks_between_words() {
        assert_eq!(wrap_line("one two three", 7), vec!["one two", "three"]);
    }

    #[test]
    fn wrap_line_wraps_long_words() {
        assert_eq!(wrap_line("abcdefgh", 3), vec!["abc", "def", "gh"]);
        assert_eq!(wrap_line("ab cdefgh", 3), vec!["ab", "cde", "fgh"]);
    }

    #[test]
    fn wrap_line_preserves_indentation() {
        assert_eq!(
            wrap_line("  one two three", 7),
            vec!["  one", "  two", "  three"]
        );
    }

    #[test]
    fn visual_rows_counts_wrapped_terminal_rows() {
        assert_eq!(visual_rows("", 10), 1);
        assert_eq!(visual_rows("abcdefghij", 10), 1);
        assert_eq!(visual_rows("abcdefghijk", 10), 2);
        assert_eq!(visual_rows("\x1b[1mabc\x1b[0m", 3), 1);
    }

    #[test]
    fn select_line_marks_selected_and_unselected_choices() {
        disable_colors();

        assert_eq!(select_line("Commit", true), "  ● Commit");
        assert_eq!(select_line("Cancel", false), "  ○ Cancel");
    }

    #[test]
    fn selected_line_uses_rail_and_muted_choice() {
        disable_colors();

        assert_eq!(selected_line("Cancel"), "│ Cancel");
    }

    #[test]
    fn confirm_label_removes_trailing_question_mark() {
        assert_eq!(confirm_label("Commit and push?"), "Commit and push");
        assert_eq!(confirm_label("Commit and push"), "Commit and push");
    }

    #[test]
    fn cancel_choice_finds_case_insensitive_cancel_label() {
        let choices = [Choice::new("Run", 1), Choice::new("cancel", 2)];

        assert_eq!(cancel_choice(&choices), Some(1));
        assert_eq!(cancel_choice(&choices[..1]), None);
    }

    #[test]
    fn digit_key_uses_one_based_indices() {
        assert_eq!(digit_key('1'), super::SelectKey::Index(0));
        assert_eq!(digit_key('3'), super::SelectKey::Index(2));
        assert_eq!(digit_key('0'), super::SelectKey::Ignore);
        assert_eq!(digit_key('x'), super::SelectKey::Ignore);
    }
}
