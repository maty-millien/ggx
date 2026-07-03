use console::{Key, Term, style};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, IsTerminal, Read, Write};
use std::time::{Duration, Instant};

pub fn step(label: &str, elapsed: Duration) {
    println!(
        "{} {} {}",
        style("+").green(),
        style(label).bold(),
        style(format!("{:.1}s", elapsed.as_secs_f32())).dim()
    );
    rail();
}

pub fn section(title: &str) {
    println!("{} {}", rail_text(), style(title).bold());
}

pub fn change_rows(rows: &[ChangeRow]) {
    for row in rows {
        println!(
            "{}   {}  {}{}{}",
            rail_text(),
            change_status(row.status),
            path(&row.path),
            addition(row.additions.as_deref()),
            deletion(row.deletions.as_deref())
        );
    }
    rail();
}

pub fn message(text: &str) {
    println!("{} {}", rail_text(), commit_message(text));
    rail();
}

pub fn block(text: &str) {
    let width = Term::stdout().size().1 as usize;
    let width = width.saturating_sub(4).max(20);

    for line in text.lines() {
        if line.is_empty() {
            rail();
            continue;
        }

        for line in wrap_line(line, width) {
            println!("{} {}", rail_text(), line);
        }
    }
    rail();
}

pub fn confirm(prompt: &str) -> anyhow::Result<bool> {
    select(
        "What would you like to do?",
        &[
            Choice::new(confirm_label(prompt), true),
            Choice::new("Cancel", false),
        ],
    )
}

const CUSTOM_LABEL: &str = "Other…";
const CANCEL_LABEL: &str = "Cancel";

#[derive(Clone, Copy)]
enum CustomChoice {
    Option(usize),
    Custom,
    Cancel,
}

pub fn input(prompt: &str) -> anyhow::Result<String> {
    let term = Term::stdout();
    println!("{} {}", style("+").green(), style(prompt).bold());
    print!("{} ", rail_text());
    io::stdout().flush()?;

    let value = read_input_line(&term)?.trim().to_string();
    rail();

    Ok(value)
}

pub fn select_with_custom(prompt: &str, options: &[&str]) -> anyhow::Result<Option<String>> {
    if io::stdin().is_terminal() {
        return select_with_custom_interactive(prompt, options);
    }

    match select(prompt, &custom_choices(options))? {
        CustomChoice::Option(index) => Ok(Some(options[index].to_string())),
        CustomChoice::Custom => Ok(Some(input(prompt)?)),
        CustomChoice::Cancel => Ok(None),
    }
}

fn select_with_custom_interactive(
    prompt: &str,
    options: &[&str],
) -> anyhow::Result<Option<String>> {
    let term = Term::stdout();
    let _cursor = CursorGuard::hide(&term)?;

    let custom_index = options.len();
    let cancel_index = options.len() + 1;
    let row_count = options.len() + 2;

    let mut selected = 0;
    let mut custom = String::new();
    let mut rendered = false;

    loop {
        if rendered {
            term.clear_last_lines(row_count + 1)?;
        }

        render_custom_select(prompt, options, selected, &custom);
        rendered = true;

        match read_custom_key(selected == custom_index)? {
            CustomKey::Confirm => {
                let value = if selected == custom_index {
                    let value = custom.trim();
                    if value.is_empty() {
                        continue;
                    }
                    Some(value.to_string())
                } else if selected == cancel_index {
                    None
                } else {
                    Some(options[selected].to_string())
                };

                finish_custom_select(&term, prompt, row_count, value.as_deref())?;
                return Ok(value);
            }
            CustomKey::Cancel => {
                finish_custom_select(&term, prompt, row_count, None)?;
                return Ok(None);
            }
            CustomKey::Next => selected = (selected + 1) % row_count,
            CustomKey::Previous => {
                selected = if selected == 0 {
                    row_count - 1
                } else {
                    selected - 1
                };
            }
            CustomKey::Jump(index) if index < row_count => selected = index,
            CustomKey::Insert(character) => custom.push(character),
            CustomKey::Backspace => {
                custom.pop();
            }
            CustomKey::Jump(_) | CustomKey::Ignore => {}
        }
    }
}

fn custom_choices<'a>(options: &[&'a str]) -> Vec<Choice<'a, CustomChoice>> {
    let mut choices = options
        .iter()
        .enumerate()
        .map(|(index, option)| Choice::new(option, CustomChoice::Option(index)))
        .collect::<Vec<_>>();
    choices.push(Choice::new(CUSTOM_LABEL, CustomChoice::Custom));
    choices.push(Choice::new(CANCEL_LABEL, CustomChoice::Cancel));

    choices
}

fn read_input_line(term: &Term) -> anyhow::Result<String> {
    if io::stdin().is_terminal() {
        return Ok(term.read_line()?);
    }

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;

    Ok(line)
}

pub fn select<T: Clone>(prompt: &str, choices: &[Choice<'_, T>]) -> anyhow::Result<T> {
    anyhow::ensure!(!choices.is_empty(), "select requires at least one choice");

    let term = Term::stdout();
    let _cursor = CursorGuard::hide(&term)?;
    let mut selected = 0;
    let mut rendered = false;

    loop {
        if rendered && io::stdin().is_terminal() {
            term.clear_last_lines(choices.len() + 1)?;
        }

        render_select(prompt, choices, selected);
        rendered = true;

        match read_select_key()? {
            SelectKey::Confirm => {
                finish_select(&term, prompt, choices, selected, rendered)?;
                return Ok(choices[selected].value.clone());
            }
            SelectKey::Cancel => {
                if let Some(index) = cancel_choice(choices) {
                    finish_select(&term, prompt, choices, index, rendered)?;
                    return Ok(choices[index].value.clone());
                }
            }
            SelectKey::Next => {
                selected = (selected + 1) % choices.len();
            }
            SelectKey::Previous => {
                selected = if selected == 0 {
                    choices.len() - 1
                } else {
                    selected - 1
                };
            }
            SelectKey::Index(index) if index < choices.len() => {
                selected = index;
            }
            SelectKey::Index(_) | SelectKey::Ignore => {}
        }
    }
}

pub fn spinner<T>(
    message: &'static str,
    operation: impl FnOnce() -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")?.tick_strings(&["◐", "◓", "◑", "◒"]),
    );
    spinner.set_message(style(message).bold().to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));

    let result = operation();
    spinner.finish_and_clear();

    result
}

pub fn timed_spinner<T>(
    message: &'static str,
    operation: impl FnOnce() -> anyhow::Result<T>,
) -> anyhow::Result<(T, Duration)> {
    let started = Instant::now();
    let value = spinner(message, operation)?;

    Ok((value, started.elapsed()))
}

pub fn success(label: &str, value: &str) {
    println!(
        "{} {} {}",
        style("+").green(),
        style(label).green().bold(),
        style(value).cyan()
    );
}

pub fn warning(text: &str) {
    println!("{} {}", style("+").green(), style(text).yellow().bold());
}

pub fn aborted() {
    println!("{} {}", style("+").red(), style("Aborted").red().bold());
}

pub fn error(error: &anyhow::Error) {
    eprintln!("{} {}", style("+").red(), style(error).red().bold());
}

pub fn rail() {
    println!("{}", rail_text());
}

fn rail_text() -> console::StyledObject<&'static str> {
    style("│").dim()
}

fn change_status(status: ChangeStatus) -> console::StyledObject<&'static str> {
    match status {
        ChangeStatus::Added => style("A").green(),
        ChangeStatus::Modified => style("M").yellow(),
        ChangeStatus::Deleted => style("D").red(),
        ChangeStatus::Renamed => style("R").cyan(),
        ChangeStatus::Unknown => style("?").dim(),
    }
}

fn path(path: &str) -> String {
    if path.ends_with('/') {
        return style(path).bold().to_string();
    }

    let Some((dir, file)) = path.rsplit_once('/') else {
        return style(path).bold().to_string();
    };

    format!("{}/{}", style(dir).dim(), style(file).bold())
}

fn addition(value: Option<&str>) -> String {
    match value {
        Some("-") | None => String::new(),
        Some("0") => String::new(),
        Some(value) => format!(" {}", style(format!("+{}", value)).green()),
    }
}

fn deletion(value: Option<&str>) -> String {
    match value {
        Some("-") | None => String::new(),
        Some("0") => String::new(),
        Some(value) => format!(" {}", style(format!("-{}", value)).red()),
    }
}

fn commit_message(message: &str) -> String {
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

fn wrap_line(line: &str, width: usize) -> Vec<String> {
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

fn render_select<T>(prompt: &str, choices: &[Choice<'_, T>], selected: usize) {
    println!("{} {}", style("+").green(), style(prompt).bold());

    for (index, choice) in choices.iter().enumerate() {
        println!("{}", select_line(choice.label, index == selected));
    }
}

fn finish_select<T>(
    term: &Term,
    prompt: &str,
    choices: &[Choice<'_, T>],
    selected: usize,
    rendered: bool,
) -> anyhow::Result<()> {
    if rendered && io::stdin().is_terminal() {
        term.clear_last_lines(choices.len() + 1)?;
    }

    println!("{} {}", style("+").green(), style(prompt).bold());
    println!("{}", selected_line(choices[selected].label));
    rail();

    Ok(())
}

fn render_custom_select(prompt: &str, options: &[&str], selected: usize, custom: &str) {
    println!("{} {}", style("+").green(), style(prompt).bold());

    for (index, option) in options.iter().enumerate() {
        println!("{}", select_line(option, index == selected));
    }

    println!("{}", custom_line(custom, selected == options.len()));
    println!("{}", select_line(CANCEL_LABEL, selected == options.len() + 1));
}

fn finish_custom_select(
    term: &Term,
    prompt: &str,
    row_count: usize,
    value: Option<&str>,
) -> anyhow::Result<()> {
    term.clear_last_lines(row_count + 1)?;

    println!("{} {}", style("+").green(), style(prompt).bold());
    println!("{}", selected_line(value.unwrap_or(CANCEL_LABEL)));
    rail();

    Ok(())
}

fn selected_line(label: &str) -> String {
    format!("{} {}", rail_text(), style(label).dim())
}

fn custom_line(custom: &str, selected: bool) -> String {
    if !selected {
        if custom.is_empty() {
            return format!("  {} {}", style("○").dim(), style(CUSTOM_LABEL).dim());
        }

        return format!(
            "  {} {}",
            style("○").dim(),
            style(format!("Other: {}", custom)).dim()
        );
    }

    format!(
        "  {} {}{}{}",
        style("●").green(),
        style("Other: ").bold(),
        style(custom).bold(),
        style("▏").green()
    )
}

fn select_line(label: &str, selected: bool) -> String {
    if selected {
        return format!("  {} {}", style("●").green(), style(label).bold());
    }

    format!("  {} {}", style("○").dim(), style(label).dim())
}

fn confirm_label(prompt: &str) -> &str {
    prompt.trim().trim_end_matches('?')
}

fn cancel_choice<T>(choices: &[Choice<'_, T>]) -> Option<usize> {
    choices
        .iter()
        .position(|choice| choice.label.eq_ignore_ascii_case("cancel"))
}

fn read_select_key() -> anyhow::Result<SelectKey> {
    if io::stdin().is_terminal() {
        return Ok(match Term::stdout().read_key()? {
            Key::Enter => SelectKey::Confirm,
            Key::Escape | Key::CtrlC => SelectKey::Cancel,
            Key::ArrowDown | Key::Char('j') | Key::Char('J') => SelectKey::Next,
            Key::ArrowUp | Key::Char('k') | Key::Char('K') => SelectKey::Previous,
            Key::Char('q') | Key::Char('Q') | Key::Char('n') | Key::Char('N') => SelectKey::Cancel,
            Key::Char('y') | Key::Char('Y') => SelectKey::Confirm,
            Key::Char(character) => digit_key(character),
            _ => SelectKey::Ignore,
        });
    }

    let mut buffer = [0; 1];
    let read = io::stdin().read(&mut buffer)?;
    if read == 0 {
        return Ok(SelectKey::Confirm);
    }

    Ok(match buffer[0] as char {
        '\n' | '\r' | 'y' | 'Y' => SelectKey::Confirm,
        'n' | 'N' | 'q' | 'Q' => SelectKey::Cancel,
        character => digit_key(character),
    })
}

fn digit_key(character: char) -> SelectKey {
    character
        .to_digit(10)
        .and_then(|digit| usize::try_from(digit).ok())
        .and_then(|digit| digit.checked_sub(1))
        .map_or(SelectKey::Ignore, SelectKey::Index)
}

fn read_custom_key(editing: bool) -> anyhow::Result<CustomKey> {
    Ok(match Term::stdout().read_key()? {
        Key::Enter => CustomKey::Confirm,
        Key::Escape | Key::CtrlC => CustomKey::Cancel,
        Key::ArrowDown => CustomKey::Next,
        Key::ArrowUp => CustomKey::Previous,
        Key::Backspace if editing => CustomKey::Backspace,
        Key::Char(character) if editing => CustomKey::Insert(character),
        Key::Char('j') | Key::Char('J') => CustomKey::Next,
        Key::Char('k') | Key::Char('K') => CustomKey::Previous,
        Key::Char(character) => digit_jump(character),
        _ => CustomKey::Ignore,
    })
}

fn digit_jump(character: char) -> CustomKey {
    character
        .to_digit(10)
        .and_then(|digit| usize::try_from(digit).ok())
        .and_then(|digit| digit.checked_sub(1))
        .map_or(CustomKey::Ignore, CustomKey::Jump)
}

struct CursorGuard<'a> {
    term: &'a Term,
}

impl<'a> CursorGuard<'a> {
    fn hide(term: &'a Term) -> anyhow::Result<Self> {
        term.hide_cursor()?;
        Ok(Self { term })
    }
}

impl Drop for CursorGuard<'_> {
    fn drop(&mut self) {
        let _ = self.term.show_cursor();
    }
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
    label: &'a str,
    value: T,
}

impl<'a, T> Choice<'a, T> {
    pub fn new(label: &'a str, value: T) -> Self {
        Self { label, value }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectKey {
    Confirm,
    Cancel,
    Next,
    Previous,
    Index(usize),
    Ignore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CustomKey {
    Confirm,
    Cancel,
    Next,
    Previous,
    Jump(usize),
    Insert(char),
    Backspace,
    Ignore,
}

#[cfg(test)]
mod tests {
    use super::{
        Choice, CustomChoice, CustomKey, addition, cancel_choice, commit_message, confirm_label,
        custom_choices, custom_line, deletion, digit_jump, digit_key, path, select_line,
        selected_line, wrap_line,
    };

    fn disable_colors() {
        console::set_colors_enabled(false);
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
        assert_eq!(commit_message("plain message"), "plain message");
    }

    #[test]
    fn wrap_line_wraps_long_words() {
        assert_eq!(wrap_line("abcdefgh", 3), vec!["abc", "def", "gh"]);
    }

    #[test]
    fn wrap_line_preserves_indentation() {
        assert_eq!(
            wrap_line("  one two three", 7),
            vec!["  one", "  two", "  three"]
        );
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
    }

    #[test]
    fn digit_key_uses_one_based_indices() {
        assert_eq!(digit_key('1'), super::SelectKey::Index(0));
        assert_eq!(digit_key('3'), super::SelectKey::Index(2));
        assert_eq!(digit_key('x'), super::SelectKey::Ignore);
    }

    #[test]
    fn custom_line_shows_placeholder_until_selected() {
        disable_colors();

        assert_eq!(custom_line("", false), "  ○ Other…");
        assert_eq!(custom_line("", true), "  ● Other: ▏");
    }

    #[test]
    fn custom_line_echoes_typed_value_inline() {
        disable_colors();

        assert_eq!(custom_line("trunk", false), "  ○ Other: trunk");
        assert_eq!(custom_line("trunk", true), "  ● Other: trunk▏");
    }

    #[test]
    fn digit_jump_uses_one_based_indices() {
        assert_eq!(digit_jump('1'), CustomKey::Jump(0));
        assert_eq!(digit_jump('4'), CustomKey::Jump(3));
        assert_eq!(digit_jump('x'), CustomKey::Ignore);
    }

    #[test]
    fn custom_choices_appends_other_and_cancel() {
        let choices = custom_choices(&["Alpha", "Beta"]);

        assert_eq!(choices.len(), 4);
        assert_eq!(choices[0].label, "Alpha");
        assert!(matches!(choices[0].value, CustomChoice::Option(0)));
        assert_eq!(choices[1].label, "Beta");
        assert!(matches!(choices[1].value, CustomChoice::Option(1)));
        assert_eq!(choices[2].label, "Other…");
        assert!(matches!(choices[2].value, CustomChoice::Custom));
        assert_eq!(choices[3].label, "Cancel");
        assert!(matches!(choices[3].value, CustomChoice::Cancel));
    }
}
