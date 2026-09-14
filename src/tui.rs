use console::{Key, Term, style};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, IsTerminal, Read};
#[cfg(unix)]
use std::os::fd::{AsRawFd, RawFd};
use std::time::{Duration, Instant};

mod format;

pub use format::{ChangeRow, ChangeStatus, Choice};
use format::{
    SelectKey, addition, cancel_choice, change_status, commit_message, confirm_label, deletion,
    digit_key, path, rail_text, select_line, selected_line, visual_rows, wrap_line,
};

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
    let mut rendered = Vec::new();

    loop {
        clear_rendered(&term, &rendered)?;
        rendered = render_select(prompt, choices, selected);

        match read_select_key()? {
            SelectKey::Confirm => {
                finish_select(&term, prompt, choices, selected, &rendered)?;
                return Ok(choices[selected].value.clone());
            }
            SelectKey::Cancel => {
                if let Some(index) = cancel_choice(choices) {
                    finish_select(&term, prompt, choices, index, &rendered)?;
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

fn block_width() -> usize {
    let width = Term::stdout().size().1 as usize;
    width.saturating_sub(4).max(20)
}

fn render_select<T>(prompt: &str, choices: &[Choice<'_, T>], selected: usize) -> Vec<String> {
    let mut lines = vec![format!("{} {}", style("+").green(), style(prompt).bold())];
    lines.extend(
        choices
            .iter()
            .enumerate()
            .map(|(index, choice)| select_line(choice.label, index == selected)),
    );

    for line in &lines {
        println!("{}", line);
    }

    lines
}

fn clear_rendered(term: &Term, lines: &[String]) -> anyhow::Result<()> {
    if !lines.is_empty() && io::stdin().is_terminal() {
        let columns = term.size().1 as usize;
        let rows = lines.iter().map(|line| visual_rows(line, columns)).sum();
        term.clear_last_lines(rows)?;
    }

    Ok(())
}

fn finish_select<T>(
    term: &Term,
    prompt: &str,
    choices: &[Choice<'_, T>],
    selected: usize,
    rendered: &[String],
) -> anyhow::Result<()> {
    clear_rendered(term, rendered)?;

    println!("{} {}", style("+").green(), style(prompt).bold());
    println!("{}", selected_line(choices[selected].label));
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
