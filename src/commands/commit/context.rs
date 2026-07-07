use crate::vcs::git;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
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
        collect_for_branch_with_env(branch, &[])
    }
}

fn collect_for_branch_with_env(branch: String, envs: &[(&str, &OsStr)]) -> anyhow::Result<Context> {
    let preview = PreviewIndex::from_current(envs)?;
    preview.run(envs, &["add", "--all"])?;

    let files = preview
        .run(envs, &["diff", "--staged", "--name-status"])?
        .trim()
        .to_string();
    let stat = preview
        .run(envs, &["diff", "--staged", "--stat"])?
        .trim()
        .to_string();
    let numstat = preview
        .run(envs, &["diff", "--staged", "--numstat"])?
        .trim()
        .to_string();
    let summary = preview
        .run(envs, &["diff", "--staged", "--summary"])?
        .trim()
        .to_string();
    let diff = preview
        .run(envs, &["diff", "--staged", "--unified=3"])?
        .trim()
        .to_string();

    if files.is_empty() {
        anyhow::bail!("No changes found.");
    }

    let diff = budget_diff(diff, MAX_DIFF_CHARS);
    let repo_root = git::run_with_env(&["rev-parse", "--show-toplevel"], envs)?
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

    Ok(Context {
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

struct PreviewIndex {
    path: PathBuf,
}

impl PreviewIndex {
    fn from_current(envs: &[(&str, &OsStr)]) -> anyhow::Result<Self> {
        let index = Self {
            path: temporary_index_path(),
        };
        let current_index = git::run_with_env(&["rev-parse", "--git-path", "index"], envs)?
            .trim()
            .to_string();

        if Path::new(&current_index).exists() {
            fs::copy(current_index, &index.path)?;
        } else {
            let _ = index.run(envs, &["read-tree", "HEAD"]);
        }

        Ok(index)
    }

    fn run(&self, envs: &[(&str, &OsStr)], args: &[&str]) -> anyhow::Result<String> {
        let mut envs = envs.to_vec();
        envs.push(("GIT_INDEX_FILE", self.path.as_os_str()));

        git::run_with_env(args, &envs)
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
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!("ggx-index-{}-{}", std::process::id(), nanos))
}

struct BudgetedDiff {
    value: String,
    total_truncated: bool,
    file_truncated: bool,
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

fn budget_diff(value: String, max_chars: usize) -> BudgetedDiff {
    if value.chars().count() <= max_chars {
        return BudgetedDiff {
            value,
            total_truncated: false,
            file_truncated: false,
        };
    }

    if max_chars == 0 {
        return BudgetedDiff {
            value: String::new(),
            total_truncated: true,
            file_truncated: true,
        };
    }

    let sections = split_diff_sections(&value);
    let section_count = sections.len();
    let mut allocations = vec![0; section_count];

    if section_count == 0 {
        return BudgetedDiff {
            value: take_chars(&value, max_chars),
            total_truncated: true,
            file_truncated: true,
        };
    }

    let base_budget = (max_chars / section_count).max(1);
    let mut remaining = max_chars;

    for (index, section) in sections.iter().enumerate() {
        let remaining_sections = section_count - index;
        let section_budget = base_budget.min(remaining / remaining_sections).max(1);
        let allocated = section.chars().count().min(section_budget).min(remaining);
        allocations[index] = allocated;
        remaining -= allocated;
    }

    while remaining > 0 {
        let mut spent = false;
        for (index, section) in sections.iter().enumerate() {
            let section_len = section.chars().count();
            if allocations[index] < section_len {
                allocations[index] += 1;
                remaining -= 1;
                spent = true;
                if remaining == 0 {
                    break;
                }
            }
        }

        if !spent {
            break;
        }
    }

    let file_truncated = sections
        .iter()
        .zip(allocations.iter())
        .any(|(section, allocation)| section.chars().count() > *allocation);
    let value = sections
        .iter()
        .zip(allocations.iter())
        .map(|(section, allocation)| take_chars(section, *allocation))
        .collect::<String>();

    BudgetedDiff {
        value,
        total_truncated: true,
        file_truncated,
    }
}

fn split_diff_sections(value: &str) -> Vec<&str> {
    let mut starts = value
        .match_indices("diff --git ")
        .filter_map(|(index, _)| {
            if index == 0 || value[..index].ends_with('\n') {
                Some(index)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if starts.is_empty() {
        return vec![value];
    }

    if starts[0] != 0 {
        starts.insert(0, 0);
    }

    starts
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let end = starts.get(index + 1).copied().unwrap_or(value.len());
            &value[*start..end]
        })
        .collect()
}

fn take_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::{budget_diff, collect_for_branch_with_env, truncate};
    use std::ffi::OsStr;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn budget_diff_keeps_single_small_diff() {
        let diff = String::from("diff --git a/a b/a\n+hello\n");

        let budgeted = budget_diff(diff.clone(), 100);

        assert_eq!(budgeted.value, diff);
        assert!(!budgeted.total_truncated);
        assert!(!budgeted.file_truncated);
    }

    #[test]
    fn budget_diff_represents_multiple_files_under_tight_budget() {
        let diff = [
            "diff --git a/a b/a\n",
            "@@ -1 +1 @@\n",
            "+aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
            "diff --git a/b b/b\n",
            "@@ -1 +1 @@\n",
            "+bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n",
        ]
        .concat();

        let budgeted = budget_diff(diff, 80);

        assert!(budgeted.value.contains("diff --git a/a b/a"));
        assert!(budgeted.value.contains("diff --git a/b b/b"));
        assert!(budgeted.total_truncated);
        assert!(budgeted.file_truncated);
        assert!(budgeted.value.chars().count() <= 80);
    }

    #[test]
    fn budget_diff_truncates_large_single_file_diff() {
        let diff = format!("diff --git a/a b/a\n{}", "+hello\n".repeat(100));

        let budgeted = budget_diff(diff, 60);

        assert!(budgeted.value.starts_with("diff --git a/a b/a"));
        assert!(budgeted.total_truncated);
        assert!(budgeted.file_truncated);
        assert!(budgeted.value.chars().count() <= 60);
    }

    #[test]
    fn budget_diff_keeps_empty_diff_empty() {
        let budgeted = budget_diff(String::new(), 60);

        assert!(budgeted.value.is_empty());
        assert!(!budgeted.total_truncated);
        assert!(!budgeted.file_truncated);
    }

    #[test]
    fn budget_diff_handles_zero_budget() {
        let budgeted = budget_diff("diff --git a/a b/a\n+hello\n".to_string(), 0);

        assert!(budgeted.value.is_empty());
        assert!(budgeted.total_truncated);
        assert!(budgeted.file_truncated);
    }

    #[test]
    fn budget_diff_truncates_plain_text_without_diff_sections() {
        let budgeted = budget_diff("plain text diff".to_string(), 5);

        assert_eq!(budgeted.value, "plain");
        assert!(budgeted.total_truncated);
        assert!(budgeted.file_truncated);
    }

    #[test]
    fn truncate_tracks_char_boundary() {
        let (value, truncated) = truncate("éclair".to_string(), 2);

        assert_eq!(value, "éc");
        assert!(truncated);
    }

    #[test]
    fn preview_includes_staged_and_unstaged_changes_without_changing_index() {
        let repo = TempRepo::new();
        repo.write("staged.txt", "base\n");
        repo.write("unstaged.txt", "base\n");
        repo.git(&["add", "--all"]);
        repo.git(&["commit", "-m", "initial"]);

        repo.write("staged.txt", "staged\n");
        repo.git(&["add", "staged.txt"]);
        repo.write("unstaged.txt", "unstaged\n");

        let before = repo.git(&["diff", "--staged", "--name-status"]);
        let context = collect_for_branch_with_env("feature".to_string(), &repo.envs()).unwrap();
        let after = repo.git(&["diff", "--staged", "--name-status"]);

        assert!(context.files.contains("M\tstaged.txt"));
        assert!(context.files.contains("M\tunstaged.txt"));
        assert_eq!(after, before);
    }

    #[test]
    fn preview_includes_untracked_files_without_changing_index() {
        let repo = TempRepo::new();
        repo.write("tracked.txt", "base\n");
        repo.git(&["add", "--all"]);
        repo.git(&["commit", "-m", "initial"]);
        repo.write("untracked.txt", "new\n");

        let context = collect_for_branch_with_env("feature".to_string(), &repo.envs()).unwrap();
        let staged = repo.git(&["diff", "--staged", "--name-status"]);

        assert!(context.files.contains("A\tuntracked.txt"));
        assert!(staged.trim().is_empty());
    }

    struct TempRepo {
        path: PathBuf,
        git_dir: PathBuf,
    }

    impl TempRepo {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "ggx-test-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0)
            ));
            fs::create_dir(&path).unwrap();

            let repo = Self {
                git_dir: path.join(".git"),
                path,
            };
            repo.git(&["init"]);
            repo.git(&["config", "user.email", "test@example.com"]);
            repo.git(&["config", "user.name", "Test User"]);

            repo
        }

        fn envs(&self) -> Vec<(&str, &OsStr)> {
            vec![
                ("GIT_DIR", self.git_dir.as_os_str()),
                ("GIT_WORK_TREE", self.path.as_os_str()),
            ]
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.path.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }

        fn git(&self, args: &[&str]) -> String {
            let output = Command::new("git")
                .arg("-C")
                .arg(&self.path)
                .args(args)
                .output()
                .unwrap();

            if !output.status.success() {
                panic!(
                    "git {} failed: {}",
                    args.join(" "),
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            String::from_utf8_lossy(&output.stdout).to_string()
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = remove_dir_all(&self.path);
        }
    }

    fn remove_dir_all(path: &Path) -> std::io::Result<()> {
        if path.exists() {
            fs::remove_dir_all(path)?;
        }

        Ok(())
    }
}
