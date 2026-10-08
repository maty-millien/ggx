use console::{Color, Key, Term, style};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, IsTerminal, Read};
use std::os::fd::{AsRawFd, RawFd};
use std::time::{Duration, Instant};

pub struct ChangeRow {
    pub status: char,
    pub path: String,
    pub additions: String,
    pub deletions: String,
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

enum SelectKey {
    Confirm,
    Cancel,
    Next,
    Previous,
    Index(usize),
    Ignore,
}

pub fn session<T>(interactive: bool, operation: impl FnOnce() -> T) -> T {
    let _session = interactive.then(TerminalSession::start);

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
    section("Changes");
    for row in rows {
        let status = match row.status {
            'A' => style('A').green(),
            'M' => style('M').yellow(),
            'D' => style('D').red(),
            'R' => style('R').cyan(),
            _ => style('?').dim(),
        };
        let path = match row.path.rsplit_once('/') {
            Some((dir, file)) => format!("{}/{}", style(dir).dim(), style(file).bold()),
            None => style(&row.path).bold().to_string(),
        };
        println!(
            "{}   {}  {}{}{}",
            rail_text(),
            status,
            path,
            count('+', &row.additions, Color::Green),
            count('-', &row.deletions, Color::Red)
        );
    }
    rail();
}

pub fn message(text: &str) {
    for (index, line) in wrap_line(text, block_width()).into_iter().enumerate() {
        let styled = if index == 0 {
            commit_message(&line)
        } else {
            style(line).white().to_string()
        };
        println!("{} {}", rail_text(), styled);
    }
    rail();
}

pub fn block(text: &str) {
    let width = block_width();

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

pub fn confirm(yes: bool, prompt: &str) -> anyhow::Result<bool> {
    if yes {
        return Ok(true);
    }

    select(
        "What would you like to do?",
        &[
            Choice::new(prompt.trim().trim_end_matches('?'), true),
            Choice::new("Cancel", false),
        ],
    )
}

pub fn select<T: Clone>(prompt: &str, choices: &[Choice<'_, T>]) -> anyhow::Result<T> {
    let term = Term::stdout();
    flush_pending_input();
    let mut selected = 0;
    let mut rendered = Vec::new();

    loop {
        clear_rendered(&term, &rendered)?;
        rendered = render_select(prompt, choices, selected);

        match read_select_key()? {
            SelectKey::Confirm => {
                finish_select(&term, prompt, choices[selected].label, &rendered)?;
                return Ok(choices[selected].value.clone());
            }
            SelectKey::Cancel => {
                if let Some(cancel) = choices
                    .iter()
                    .find(|choice| choice.label.eq_ignore_ascii_case("cancel"))
                {
                    finish_select(&term, prompt, cancel.label, &rendered)?;
                    return Ok(cancel.value.clone());
                }
            }
            SelectKey::Next => selected = (selected + 1) % choices.len(),
            SelectKey::Previous => selected = (selected + choices.len() - 1) % choices.len(),
            SelectKey::Index(index) if index < choices.len() => selected = index,
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

fn block_width() -> usize {
    let width = Term::stdout().size().1 as usize;
    width.saturating_sub(4).max(20)
}

fn count(sign: char, value: &str, color: Color) -> String {
    if matches!(value, "" | "-" | "0") {
        return String::new();
    }
    format!(" {}", style(format!("{sign}{value}")).fg(color))
}

fn commit_message(message: &str) -> String {
    let Some((kind, rest)) = message.split_once(':') else {
        return style(message).green().bold().to_string();
    };
    let kind = match kind.split_once('(') {
        Some((name, scope)) => format!("{}({}", style(name).green().bold(), style(scope).cyan()),
        None => style(kind).green().bold().to_string(),
    };

    format!("{kind}:{}", style(rest).white())
}

fn wrap_line(line: &str, width: usize) -> Vec<String> {
    if line.chars().count() <= width {
        return vec![line.to_string()];
    }

    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let width = width.saturating_sub(indent.chars().count()).max(1);
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in line.split_whitespace() {
        for piece in word.chars().collect::<Vec<_>>().chunks(width) {
            if !current.is_empty() && current.chars().count() + 1 + piece.len() > width {
                lines.push(format!("{indent}{current}"));
                current.clear();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.extend(piece);
        }
    }

    if !current.is_empty() || lines.is_empty() {
        lines.push(format!("{indent}{current}"));
    }
    lines
}

fn render_select<T>(prompt: &str, choices: &[Choice<'_, T>], selected: usize) -> Vec<String> {
    let mut lines = vec![format!("{} {}", style("+").green(), style(prompt).bold())];
    lines.extend(choices.iter().enumerate().map(|(index, choice)| {
        if index == selected {
            format!("  {} {}", style("●").green(), style(choice.label).bold())
        } else {
            format!("  {} {}", style("○").dim(), style(choice.label).dim())
        }
    }));

    for line in &lines {
        println!("{}", line);
    }

    lines
}

fn clear_rendered(term: &Term, lines: &[String]) -> anyhow::Result<()> {
    if !lines.is_empty() && io::stdin().is_terminal() {
        let columns = (term.size().1 as usize).max(1);
        let rows = lines
            .iter()
            .map(|line| console::measure_text_width(line).div_ceil(columns).max(1))
            .sum();
        term.clear_last_lines(rows)?;
    }

    Ok(())
}

fn finish_select(
    term: &Term,
    prompt: &str,
    label: &str,
    rendered: &[String],
) -> anyhow::Result<()> {
    clear_rendered(term, rendered)?;

    println!("{} {}", style("+").green(), style(prompt).bold());
    println!("{} {}", rail_text(), style(label).dim());
    rail();

    Ok(())
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
    if io::stdin().read(&mut buffer)? == 0 {
        return Ok(SelectKey::Confirm);
    }

    Ok(match buffer[0] as char {
        '\n' | '\r' | 'y' | 'Y' => SelectKey::Confirm,
        'n' | 'N' | 'q' | 'Q' => SelectKey::Cancel,
        character => digit_key(character),
    })
}

fn digit_key(character: char) -> SelectKey {
    match character.to_digit(10) {
        Some(digit @ 1..) => SelectKey::Index(digit as usize - 1),
        _ => SelectKey::Ignore,
    }
}

struct TerminalSession {
    term: Option<Term>,
    _input: Option<InputModeGuard>,
}

impl TerminalSession {
    fn start() -> Self {
        let term = io::stdout()
            .is_terminal()
            .then(Term::stdout)
            .filter(|term| term.hide_cursor().is_ok());

        Self {
            term,
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

fn flush_pending_input() {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        unsafe {
            libc::tcflush(stdin.as_raw_fd(), libc::TCIFLUSH);
        }
    }
}

struct InputModeGuard {
    fd: RawFd,
    original: libc::termios,
}

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

impl Drop for InputModeGuard {
    fn drop(&mut self) {
        unsafe {
            libc::tcflush(self.fd, libc::TCIFLUSH);
            libc::tcsetattr(self.fd, libc::TCSADRAIN, &self.original);
        }
    }
}
