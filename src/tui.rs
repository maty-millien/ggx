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

#[cfg(test)]
mod tests {
    use super::{addition, commit_message, deletion, path, wrap_line};

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
}
