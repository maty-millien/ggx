use console::{Term, style};
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

pub fn confirm(prompt: &str) -> anyhow::Result<bool> {
    print!("{} {} [Y/n] ", style("+").green(), style(prompt).bold());
    io::stdout().flush()?;

    loop {
        match read_confirm_char()? {
            '\n' => {
                println!();
                rail();
                return Ok(true);
            }
            'y' | 'Y' => {
                println!();
                rail();
                return Ok(true);
            }
            'n' | 'N' => {
                println!();
                rail();
                return Ok(false);
            }
            _ => {}
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

fn read_confirm_char() -> anyhow::Result<char> {
    if io::stdin().is_terminal() {
        return Ok(Term::stdout().read_char()?);
    }

    let mut buffer = [0; 1];
    let read = io::stdin().read(&mut buffer)?;
    if read == 0 {
        return Ok('\n');
    }

    Ok(buffer[0] as char)
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
