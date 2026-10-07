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

fn commit_type(kind: &str) -> String {
    let Some((name, scope)) = kind.split_once('(') else {
        return style(kind).green().bold().to_string();
    };

    format!("{}({}", style(name).green().bold(), style(scope).cyan())
}

pub(super) fn wrap_line(line: &str, width: usize) -> Vec<String> {
    if line.chars().count() <= width {
        return vec![line.to_string()];
    }

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

fn split_word(word: &str, width: usize) -> Vec<String> {
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

#[derive(Clone, Copy)]
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

pub(super) enum SelectKey {
    Confirm,
    Cancel,
    Next,
    Previous,
    Index(usize),
    Ignore,
}
