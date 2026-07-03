use crate::ai;
use crate::commands::init::{prompt, wizard};
use crate::tui;
use crate::vcs::git;
use std::fs;
use std::path::{Path, PathBuf};

const CONTRIBUTING_FILE: &str = "CONTRIBUTING.md";
const CONFIG_FILE: &str = ".ggx.json";
const MAX_README_CHARS: usize = 8_000;
const README_DIRS: &[&str] = &[".", "docs"];
const README_NAMES: &[&str] = &["readme.md", "readme.markdown", "readme.txt", "readme"];

pub fn run() -> anyhow::Result<()> {
    let repo_root = PathBuf::from(
        git::run(&["rev-parse", "--show-toplevel"])?
            .trim()
            .to_string(),
    );

    let contributing_path = repo_root.join(CONTRIBUTING_FILE);
    let config_path = repo_root.join(CONFIG_FILE);

    if !confirm_overwrite(&contributing_path, &config_path)? {
        tui::aborted();
        return Ok(());
    }

    tui::section("Contribution setup");

    let Some(settings) = wizard::run(&base_branch_candidates())? else {
        tui::aborted();
        return Ok(());
    };

    fs::write(&config_path, settings.to_json_string()?)?;
    tui::success("Created", CONFIG_FILE);
    tui::rail();

    let project_name = project_name(&repo_root);
    let readme = read_readme(&repo_root);
    let recent_commits = recent_commits();
    let ai_prompt = prompt::render(&settings, &project_name, readme.as_deref(), &recent_commits);

    let (document, elapsed) =
        tui::timed_spinner("Drafting CONTRIBUTING.md", || draft_document(&ai_prompt))?;
    tui::step("Draft ready", elapsed);
    tui::block(&document);

    if tui::confirm("Save CONTRIBUTING.md?")? {
        fs::write(&contributing_path, format!("{}\n", document.trim()))?;
        tui::success("Created", CONTRIBUTING_FILE);
    } else {
        tui::aborted();
    }

    Ok(())
}

fn confirm_overwrite(contributing: &Path, config: &Path) -> anyhow::Result<bool> {
    let mut existing = Vec::new();
    if config.exists() {
        existing.push(CONFIG_FILE);
    }
    if contributing.exists() {
        existing.push(CONTRIBUTING_FILE);
    }

    if existing.is_empty() {
        return Ok(true);
    }

    tui::confirm(&format!(
        "{} already exists. Overwrite?",
        existing.join(" and ")
    ))
}

fn base_branch_candidates() -> Vec<String> {
    let mut names = git::local_branches()
        .map(|branches| {
            branches
                .into_iter()
                .map(|branch| branch.name)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if let Ok(default_base) = git::default_base() {
        if let Some(position) = names.iter().position(|name| name == &default_base) {
            names.remove(position);
        }
        names.insert(0, default_base);
    }

    names
}

fn draft_document(prompt: &str) -> anyhow::Result<String> {
    let first = ai::generate(prompt)?;
    if !first.trim().is_empty() {
        return Ok(first);
    }

    let second = ai::generate(prompt)?;
    if second.trim().is_empty() {
        anyhow::bail!("AI returned an empty CONTRIBUTING.md draft");
    }

    Ok(second)
}

fn project_name(repo_root: &Path) -> String {
    repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("this project")
        .to_string()
}

fn recent_commits() -> String {
    git::run(&["log", "--oneline", "-n", "20"])
        .map(|output| output.trim().to_string())
        .unwrap_or_default()
}

fn read_readme(repo_root: &Path) -> Option<String> {
    let path = find_readme(repo_root)?;
    let content = fs::read_to_string(path).ok()?;

    Some(content.chars().take(MAX_README_CHARS).collect())
}

fn find_readme(root: &Path) -> Option<PathBuf> {
    README_DIRS
        .iter()
        .find_map(|dir| readme_in_dir(&root.join(dir)))
}

fn readme_in_dir(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    README_NAMES.iter().find_map(|name| {
        entries.iter().find_map(|entry| {
            let file_name = entry.file_name();
            let file_name = file_name.to_str()?;
            file_name.eq_ignore_ascii_case(name).then(|| entry.path())
        })
    })
}
