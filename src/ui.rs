use console::{Term, style};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, IsTerminal, Read, Write};
use std::time::{Duration, Instant};

pub struct Tui;

impl Tui {
    pub fn new() -> Self {
        Self
    }

    pub fn step(&self, label: &str, elapsed: Duration) {
        println!(
            "{} {} {}",
            style("+").green(),
            style(label).bold(),
            style(format!("{:.1}s", elapsed.as_secs_f32())).dim()
        );
        self.rail();
    }

    pub fn section(&self, title: &str) {
        println!("{} {}", self.rail_text(), style(title).bold());
    }

    pub fn change_rows(&self, rows: &[ChangeRow]) {
        for row in rows {
            println!(
                "{}   {}  {}{}{}",
                self.rail_text(),
                self.change_status(row.status),
                self.path(&row.path),
                self.addition(row.additions.as_deref()),
                self.deletion(row.deletions.as_deref())
            );
        }
        self.rail();
    }

    pub fn message(&self, text: &str) {
        println!("{} {}", self.rail_text(), self.commit_message(text));
        self.rail();
    }

    pub fn confirm(&self, prompt: &str) -> anyhow::Result<bool> {
        loop {
            print!("{} {} [Y/n] ", self.rail_text(), style(prompt).bold());
            io::stdout().flush()?;

            match read_confirm_char()? {
                '\n' => {
                    println!();
                    self.rail();
                    return Ok(true);
                }
                'y' | 'Y' => {
                    println!("y");
                    self.rail();
                    return Ok(true);
                }
                'n' | 'N' => {
                    println!("n");
                    self.rail();
                    return Ok(false);
                }
                _ => {
                    println!();
                    self.warning("Please press y or n.");
                }
            }
        }
    }

    pub fn spinner<T>(
        &self,
        message: &'static str,
        operation: impl FnOnce() -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")?
                .tick_strings(&["-", "\\", "|", "/"]),
        );
        spinner.set_message(message);
        spinner.enable_steady_tick(Duration::from_millis(80));

        let result = operation();
        spinner.finish_and_clear();

        result
    }

    pub fn timed_spinner<T>(
        &self,
        message: &'static str,
        operation: impl FnOnce() -> anyhow::Result<T>,
    ) -> anyhow::Result<(T, Duration)> {
        let started = Instant::now();
        let value = self.spinner(message, operation)?;

        Ok((value, started.elapsed()))
    }

    pub fn success(&self, label: &str, value: &str) {
        println!("{} {}.", style(label).green().bold(), style(value).cyan());
    }

    pub fn warning(&self, text: &str) {
        println!("{}", style(text).yellow());
    }

    fn rail(&self) {
        println!("{}", self.rail_text());
    }

    fn rail_text(&self) -> console::StyledObject<&'static str> {
        style("│").dim()
    }

    fn change_status(&self, status: ChangeStatus) -> console::StyledObject<&'static str> {
        match status {
            ChangeStatus::Added => style("A").green(),
            ChangeStatus::Modified => style("M").yellow(),
            ChangeStatus::Deleted => style("D").red(),
            ChangeStatus::Renamed => style("R").cyan(),
            ChangeStatus::Unknown => style("?").dim(),
        }
    }

    fn path(&self, path: &str) -> String {
        let Some((dir, file)) = path.rsplit_once('/') else {
            return style(path).bold().to_string();
        };

        format!("{}/{}", style(dir).dim(), style(file).bold())
    }

    fn addition(&self, value: Option<&str>) -> String {
        match value {
            Some("-") | None => String::new(),
            Some("0") => String::new(),
            Some(value) => format!(" {}", style(format!("+{}", value)).green()),
        }
    }

    fn deletion(&self, value: Option<&str>) -> String {
        match value {
            Some("-") | None => String::new(),
            Some("0") => String::new(),
            Some(value) => format!(" {}", style(format!("-{}", value)).red()),
        }
    }

    fn commit_message(&self, message: &str) -> String {
        let Some((kind, rest)) = message.split_once(':') else {
            return style(message).green().bold().to_string();
        };

        format!("{}:{}", self.commit_type(kind), style(rest).white())
    }

    fn commit_type(&self, kind: &str) -> String {
        let Some((name, scope)) = kind.split_once('(') else {
            return style(kind).green().bold().to_string();
        };

        format!("{}({}", style(name).green().bold(), style(scope).cyan())
    }
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
