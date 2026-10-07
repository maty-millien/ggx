mod diff;

use crate::vcs::git;
use diff::{budget_diff, truncate};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_DIFF_CHARS: usize = 16_000;
const MAX_README_CHARS: usize = 8_000;
const README_DIRS: &[&str] = &[".", "docs"];
const README_NAMES: &[&str] = &["readme", "readme.md", "readme.markdown", "readme.txt"];

pub struct Context {
    pub branch: String,
    pub files: String,
    pub stat: String,
    pub numstat: String,
    pub summary: String,
    pub readme: Option<String>,
    pub diff: String,
    pub diff_truncated: bool,
    pub diff_file_truncated: bool,
    pub readme_truncated: bool,
}

impl Context {
    pub(crate) fn collect_for_branch(branch: String) -> anyhow::Result<Self> {
        let preview = PreviewIndex::from_current()?;
        preview.run(&["add", "--all"])?;

        let files = preview
            .run(&["diff", "--staged", "--name-status"])?
            .trim()
            .to_string();
        let stat = preview
            .run(&["diff", "--staged", "--stat"])?
            .trim()
            .to_string();
        let numstat = preview
            .run(&["diff", "--staged", "--numstat"])?
            .trim()
            .to_string();
        let summary = preview
            .run(&["diff", "--staged", "--summary"])?
            .trim()
            .to_string();
        let diff = preview
            .run(&["diff", "--staged", "--unified=3"])?
            .trim()
            .to_string();

        if files.is_empty() {
            anyhow::bail!("No changes found.");
        }

        let diff = budget_diff(diff, MAX_DIFF_CHARS);
        let repo_root = git::run(&["rev-parse", "--show-toplevel"])?
            .trim()
            .to_string();
        let readme = read_readme(Path::new(&repo_root))?;
        let (readme, readme_truncated) = match readme {
            Some(readme) => {
                let (readme, truncated) = truncate(readme, MAX_README_CHARS);
                (Some(readme), truncated)
            }
            None => (None, false),
        };

        Ok(Self {
            branch,
            files,
            stat,
            numstat,
            summary,
            readme,
            diff: diff.value,
            diff_truncated: diff.total_truncated,
            diff_file_truncated: diff.file_truncated,
            readme_truncated,
        })
    }
}

struct PreviewIndex {
    path: PathBuf,
}

impl PreviewIndex {
    fn from_current() -> anyhow::Result<Self> {
        let index = Self {
            path: temporary_index_path(),
        };
        let current_index = git::run(&["rev-parse", "--git-path", "index"])?
            .trim()
            .to_string();

        if Path::new(&current_index).exists() {
            fs::copy(current_index, &index.path)?;
        } else {
            let _ = index.run(&["read-tree", "HEAD"]);
        }

        Ok(index)
    }

    fn run(&self, args: &[&str]) -> anyhow::Result<String> {
        git::run_with_env(args, &[("GIT_INDEX_FILE", self.path.as_os_str())])
    }
}

impl Drop for PreviewIndex {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let lock = PathBuf::from(format!("{}.lock", self.path.display()));
        let _ = fs::remove_file(lock);
    }
}

fn temporary_index_path() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "ggx-index-{}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed),
        nanos
    ))
}

fn read_readme(root: &Path) -> anyhow::Result<Option<String>> {
    let Some(path) = find_readme(root) else {
        return Ok(None);
    };

    Ok(Some(fs::read_to_string(path)?))
}

fn find_readme(root: &Path) -> Option<PathBuf> {
    README_DIRS
        .iter()
        .filter_map(|dir| readme_in_dir(&root.join(dir)))
        .next()
}

fn readme_in_dir(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    README_NAMES.iter().find_map(|readme_name| {
        entries.iter().find_map(|entry| {
            let file_name = entry.file_name();
            let file_name = file_name.to_str()?;
            if file_name.eq_ignore_ascii_case(readme_name) {
                Some(entry.path())
            } else {
                None
            }
        })
    })
}
