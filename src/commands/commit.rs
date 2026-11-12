use crate::ai::Provider;
use crate::commands::generation::{self, Context};
use crate::{git, tui};
use anyhow::bail;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::{env, fs, process};

pub const MAX_DIFF_CHARS: usize = 16_000;
const MAX_README_CHARS: usize = 8_000;
const README_NAMES: &[&str] = &["readme", "readme.md", "readme.markdown", "readme.txt"];
pub const LOCKFILE_EXCLUDES: &[&str] = &[
    "--",
    ":(top,exclude,glob)**/*.lock",
    ":(top,exclude,glob)**/*.lockb",
    ":(top,exclude,glob)**/package-lock.json",
    ":(top,exclude,glob)**/pnpm-lock.yaml",
    ":(top,exclude,glob)**/go.sum",
];

pub fn run(provider: Provider, all: bool, yes: bool) -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_no_conflicts()?;
    let branch = git::run(&["symbolic-ref", "--short", "HEAD"])?;
    let changes = Changes::collect(all)?;

    tui::step("Analysis complete", started.elapsed());
    tui::change_rows(&rows(&changes.files, &changes.numstat));

    let (generated, elapsed) = tui::timed_spinner("Generating commit message", || {
        let context = Context {
            current_branch: &branch,
            pending: Some(&changes),
            ..Default::default()
        };
        generation::generate(provider, &context)
    })?;
    let message = generated.commit.expect("generation requires commit");

    tui::step("Message generated", elapsed);
    tui::message(&message);

    let upstream = git::optional_upstream();
    let destination = match &upstream {
        Some(upstream) => Some(upstream.clone()),
        None if git::has_origin_remote() => Some(format!("origin/{branch}")),
        None => None,
    };
    let choice = match destination {
        Some(destination) => tui::confirm_push(
            yes,
            &format!("Commit and push to {destination}"),
            &format!("Commit to {branch} without pushing"),
        )?,
        None => tui::confirm(yes, &format!("Commit to {branch}?"))?.then_some(true),
    };
    let Some(push) = choice else {
        tui::aborted();
        return Ok(());
    };

    finish(&branch, &message, upstream, push, changes.all)
}

pub fn finish(
    branch: &str,
    message: &str,
    upstream: Option<String>,
    push: bool,
    all: bool,
) -> anyhow::Result<()> {
    if all {
        tui::spinner("Staging changes", || git::run(&["add", "--all"]))?;
    }
    tui::spinner("Creating commit", || git::run(&["commit", "-m", message]))?;
    tui::success("Committed to", branch);

    if push && (upstream.is_some() || git::has_origin_remote()) {
        tui::rail();
        self::push(branch, upstream, "Pushing commit")?;
    }

    Ok(())
}

pub fn push(branch: &str, upstream: Option<String>, label: &'static str) -> anyhow::Result<()> {
    let destination = match upstream {
        Some(upstream) => {
            tui::spinner(label, || git::run(&["push"]))?;
            upstream
        }
        None => {
            tui::spinner(label, || git::run(&["push", "-u", "origin", branch]))?;
            format!("origin/{branch}")
        }
    };
    tui::success("Pushed to", &destination);

    Ok(())
}

pub fn rows(files: &str, numstat: &str) -> Vec<tui::ChangeRow> {
    files
        .lines()
        .map(|line| {
            let path = line.rsplit('\t').next().unwrap_or(line);
            let (additions, deletions) = numstat
                .lines()
                .find_map(|stat| {
                    let mut fields = stat.split('\t');
                    let (additions, deletions) = (fields.next()?, fields.next()?);
                    (fields.next_back()? == path)
                        .then(|| (additions.to_string(), deletions.to_string()))
                })
                .unwrap_or_default();

            tui::ChangeRow {
                status: line.chars().next().unwrap_or('?'),
                path: path.to_string(),
                additions,
                deletions,
            }
        })
        .collect()
}

pub struct Changes {
    pub files: String,
    pub stat: String,
    pub numstat: String,
    pub summary: String,
    pub readme: Option<String>,
    pub diff: String,
    pub notes: Vec<&'static str>,
    pub all: bool,
}

impl Changes {
    pub fn collect(all: bool) -> anyhow::Result<Self> {
        let all = all || git::run(&["diff", "--cached", "--quiet"]).is_ok();
        let index = TemporaryFile(env::temp_dir().join(format!("ggx-index-{}", process::id())));
        let git =
            |args: &[&str]| crate::run("git", args, &[("GIT_INDEX_FILE", index.0.as_os_str())]);
        let current_index = git::run(&["rev-parse", "--git-path", "index"])?;
        if Path::new(&current_index).exists() {
            fs::copy(&current_index, &index.0)?;
        } else {
            let _ = git(&["read-tree", "HEAD"]);
        }
        if all {
            git(&["add", "--all"])?;
        }

        let diff = |flag: &str| git(&["diff", "--staged", flag]);
        let files = diff("--name-status")?;
        if files.is_empty() {
            bail!("No changes found.");
        }

        let mut notes = Vec::new();
        let mut full_diff =
            git(&[&["diff", "--staged", "--unified=3"], LOCKFILE_EXCLUDES].concat())?;
        if full_diff.chars().count() > MAX_DIFF_CHARS {
            notes.extend([
                "Diff exceeded context budget.",
                "One or more file diffs were truncated.",
            ]);
            full_diff = budget_diff(&full_diff, MAX_DIFF_CHARS);
        }

        let root = git::run(&["rev-parse", "--show-toplevel"])?;
        let readme = read_readme(Path::new(&root))?.map(|readme| {
            if readme.chars().count() > MAX_README_CHARS {
                notes.push("README was truncated.");
            }
            readme.chars().take(MAX_README_CHARS).collect()
        });

        Ok(Self {
            stat: diff("--stat")?,
            numstat: diff("--numstat")?,
            summary: diff("--summary")?,
            files,
            readme,
            diff: full_diff,
            notes,
            all,
        })
    }
}

struct TemporaryFile(PathBuf);

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn budget_diff(diff: &str, max_chars: usize) -> String {
    let mut starts: Vec<usize> = std::iter::once(0)
        .chain(
            diff.match_indices("\ndiff --git ")
                .map(|(index, _)| index + 1),
        )
        .collect();
    starts.push(diff.len());
    let sections: Vec<&str> = starts
        .windows(2)
        .map(|pair| &diff[pair[0]..pair[1]])
        .collect();

    let mut by_size: Vec<usize> = (0..sections.len()).collect();
    by_size.sort_by_key(|&index| sections[index].chars().count());
    let mut budgets = vec![0; sections.len()];
    let mut remaining = max_chars;
    for (done, &index) in by_size.iter().enumerate() {
        budgets[index] = sections[index]
            .chars()
            .count()
            .min(remaining / (sections.len() - done));
        remaining -= budgets[index];
    }

    sections
        .iter()
        .zip(budgets)
        .flat_map(|(section, budget)| section.chars().take(budget))
        .collect()
}

fn read_readme(root: &Path) -> anyhow::Result<Option<String>> {
    for dir in [root.to_path_buf(), root.join("docs")] {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let paths: Vec<PathBuf> = entries
            .filter_map(|entry| Some(entry.ok()?.path()))
            .collect();
        for name in README_NAMES {
            if let Some(path) = paths.iter().find(|path| {
                path.file_name()
                    .and_then(|file| file.to_str())
                    .is_some_and(|file| file.eq_ignore_ascii_case(name))
            }) {
                return Ok(Some(fs::read_to_string(path)?));
            }
        }
    }

    Ok(None)
}
