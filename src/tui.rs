use console::{Key, Term, style};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, IsTerminal, Read};
#[cfg(unix)]
use std::os::fd::{AsRawFd, RawFd};
use std::time::{Duration, Instant};

pub fn session<T>(operation: impl FnOnce() -> T) -> T {
    let _session = TerminalSession::start();

    operation()
}

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

pub fn select<T: Clone>(prompt: &str, choices: &[Choice<'_, T>]) -> anyhow::Result<T> {
    anyhow::ensure!(!choices.is_empty(), "select requires at least one choice");

    let term = Term::stdout();
    flush_pending_input();
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

fn selected_line(label: &str) -> String {
    format!("{} {}", rail_text(), style(label).dim())
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

struct TerminalSession {
    term: Option<Term>,
    #[cfg(unix)]
    _input: Option<InputModeGuard>,
}

impl TerminalSession {
    fn start() -> Self {
        let term = hide_cursor();

        Self {
            term,
            #[cfg(unix)]
            _input: InputModeGuard::disable_echo(),
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if let Some(term) = &self.term {
            let _ = term.show_cursor();
        }
    }
}

fn hide_cursor() -> Option<Term> {
    if !io::stdout().is_terminal() {
        return None;
    }

    let term = Term::stdout();
    term.hide_cursor().ok()?;
    Some(term)
}

#[cfg(unix)]
fn flush_pending_input() {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        unsafe {
            libc::tcflush(stdin.as_raw_fd(), libc::TCIFLUSH);
        }
    }
}

#[cfg(not(unix))]
fn flush_pending_input() {}

#[cfg(unix)]
struct InputModeGuard {
    fd: RawFd,
    original: libc::termios,
}

#[cfg(unix)]
impl InputModeGuard {
    fn disable_echo() -> Option<Self> {
        let stdin = io::stdin();
        if !stdin.is_terminal() {
            return None;
        }

        let fd = stdin.as_raw_fd();
        let mut termios = std::mem::MaybeUninit::uninit();
        if unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) } != 0 {
            return None;
        }

        let original = unsafe { termios.assume_init() };
        let mut updated = original;
        updated.c_lflag &= !(libc::ECHO | libc::ECHONL);

        if unsafe { libc::tcsetattr(fd, libc::TCSADRAIN, &updated) } != 0 {
            return None;
        }

        Some(Self { fd, original })
    }
}

#[cfg(unix)]
impl Drop for InputModeGuard {
    fn drop(&mut self) {
        unsafe {
            libc::tcflush(self.fd, libc::TCIFLUSH);
            libc::tcsetattr(self.fd, libc::TCSADRAIN, &self.original);
        }
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

#[cfg(test)]
mod tests {
    use super::{
        Choice, addition, cancel_choice, commit_message, confirm_label, deletion, digit_key, path,
        select_line, selected_line, wrap_line,
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
}
