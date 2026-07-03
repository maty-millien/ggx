use crate::commands::init::config::{Decision, Settings};
use crate::tui;

pub fn run(base_branches: &[String]) -> anyhow::Result<Option<Settings>> {
    let Some(commit_convention) = tui::select_with_custom(
        "Which commit convention should contributors follow?",
        &["Conventional Commits", "Free-form"],
    )?
    else {
        return Ok(None);
    };

    let Some(squash_on_merge) = choose_decision("Squash pull requests when merging?")? else {
        return Ok(None);
    };

    let Some(base_branch) = choose_base_branch(base_branches)? else {
        return Ok(None);
    };

    let Some(open_as_draft) = choose_decision("Open pull requests as drafts by default?")? else {
        return Ok(None);
    };

    let Some(push_policy) = tui::select_with_custom(
        "What is the push policy for contributors?",
        &["PR-only", "Direct push"],
    )?
    else {
        return Ok(None);
    };

    Ok(Some(Settings {
        commit_convention,
        squash_on_merge,
        base_branch,
        open_as_draft,
        push_policy,
    }))
}

fn choose_base_branch(branches: &[String]) -> anyhow::Result<Option<String>> {
    if branches.is_empty() {
        return tui::input("Which branch do pull requests target?").map(Some);
    }

    let options = branches.iter().map(String::as_str).collect::<Vec<_>>();

    tui::select_with_custom("Which branch do pull requests target?", &options)
}

fn choose_decision(prompt: &str) -> anyhow::Result<Option<Decision>> {
    Ok(tui::select_with_custom(prompt, &["Yes", "No"])?.map(Decision::from_answer))
}
