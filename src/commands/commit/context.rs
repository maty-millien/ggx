use crate::git;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_DIFF_CHARS: usize = 16_000;
const MAX_README_CHARS: usize = 8_000;
const README_DIRS: &[&str] = &[".", "docs"];
const README_NAMES: &[&str] = &["readme", "readme.md", "readme.markdown", "readme.txt"];

pub struct Context {
    pub branch: String,
    pub change_source: &'static str,
    pub stage_before_commit: bool,
    pub files: String,
    pub stat: String,
    pub numstat: String,
    pub readme: Option<String>,
    pub diff: String,
    pub diff_truncated: bool,
    pub readme_truncated: bool,
}

impl Context {
    pub fn collect() -> anyhow::Result<Self> {
        let staged_files = git::staged_files()?;
        let stage_before_commit = staged_files.is_empty();
        let change_source = if stage_before_commit {
            "unstaged"
        } else {
            "staged"
        };

        let (files, stat, numstat, diff) = if stage_before_commit {
            (
                git::working_tree_status()?,
                git::unstaged_diff_stat()?,
                git::unstaged_numstat()?,
                git::unstaged_diff()?,
            )
        } else {
            (
                staged_files,
                git::staged_diff_stat()?,
                git::staged_numstat()?,
                git::staged_diff()?,
            )
        };

        let (stat, numstat, diff) = if stage_before_commit && diff.is_empty() {
            (
                String::from("Untracked-only changes; diff is empty until files are staged."),
                String::new(),
                String::new(),
            )
        } else {
            (stat, numstat, diff)
        };

        if files.is_empty() {
            anyhow::bail!("No staged or unstaged changes found.");
        }

        let (diff, diff_truncated) = truncate(diff, MAX_DIFF_CHARS);
        let readme = read_readme(Path::new(&git::repo_root()?))?;
        let (readme, readme_truncated) = match readme {
            Some(readme) => {
                let (readme, truncated) = truncate(readme, MAX_README_CHARS);
                (Some(readme), truncated)
            }
            None => (None, false),
        };

        Ok(Self {
            branch: git::current_branch()?,
            change_source,
            stage_before_commit,
            files,
            stat,
            numstat,
            readme,
            diff,
            diff_truncated,
            readme_truncated,
        })
    }

    pub fn render_prompt(&self) -> String {
        let readme = self.readme.as_ref().map_or(String::new(), |readme| {
            format!(
                r#"
## README

````markdown
{}
````
"#,
                readme
            )
        });

        let mut notes = Vec::new();
        if self.diff_truncated {
            notes.push("Diff was truncated.");
        }
        if self.readme_truncated {
            notes.push("README was truncated.");
        }
        let notes = if notes.is_empty() {
            String::new()
        } else {
            format!("\n\n## Notes\n\n{}", notes.join("\n"))
        };

        format!(
            r#"## Instructions

Generate a concise git commit message for the {change_source} changes.
Use Conventional Commits when appropriate: feat, fix, refactor, docs, test, chore, build, ci.
Return only the commit message.
No markdown.
No explanation.

## Branch

````
{}
````

## Changed Files

````
{}
````

## Diff Stat

````
{}
````
{}

## Diff

````diff
{}
````{}"#,
            self.branch,
            self.files,
            self.stat,
            readme,
            self.diff,
            notes,
            change_source = self.change_source
        )
    }
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

fn truncate(value: String, max_chars: usize) -> (String, bool) {
    if value.chars().count() <= max_chars {
        return (value, false);
    }

    let truncated = value.chars().take(max_chars).collect();

    (truncated, true)
}
