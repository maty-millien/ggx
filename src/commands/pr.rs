use crate::ai::Provider;
use crate::commands::commit::{self, Changes, MAX_DIFF_CHARS};
use crate::commands::generation::{self, Context};
use crate::{git, github, tui};
use anyhow::{bail, ensure};
use std::time::Instant;

const MAX_ISSUE_BODY_CHARS: usize = 8_000;

/// What the branch already committed since it left the base.
pub struct Committed {
    pub commits: String,
    pub files: String,
    pub stat: String,
    pub numstat: String,
    pub diff: String,
}

pub fn run(
    provider: Provider,
    draft: bool,
    closes: Vec<String>,
    requested_base: Option<String>,
    yes: bool,
) -> anyhow::Result<()> {
    let started = Instant::now();
    git::ensure_no_conflicts()?;

    let branch = git::run(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    ensure!(branch != "HEAD", "Cannot create a PR from detached HEAD.");
    let default_base = git::default_base()?;
    let base = requested_base.unwrap_or_else(|| default_base.clone());
    let has_changes = git::has_changes()?;
    if branch == default_base && base != default_base {
        bail!(
            "Current branch '{branch}' is the default base branch. Checkout '{base}' or a feature branch first."
        );
    }
    // Changes made directly on the base go to a new branch.
    let create_branch = branch == base;
    ensure!(
        !create_branch || has_changes,
        "No pending changes found on base branch '{base}'."
    );
    if !create_branch && let Some(url) = github::open_pull_request(&branch)? {
        bail!("A pull request is already open for this branch: {url}");
    }

    let base_ref = git::base_ref(&base)?;
    let committed = if create_branch {
        None
    } else {
        committed_changes(&base_ref)?
    };
    let pending = if has_changes {
        Some(Changes::collect()?)
    } else {
        None
    };
    ensure!(
        committed.is_some() || pending.is_some(),
        "No changes found between {base} and {branch}."
    );
    let issues = closes
        .iter()
        .map(|reference| {
            let mut issue = github::issue(reference)?;
            issue.body = issue.body.chars().take(MAX_ISSUE_BODY_CHARS).collect();
            Ok(issue)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    tui::step("Analysis complete", started.elapsed());
    let mut rows = committed.as_ref().map_or_else(Vec::new, |committed| {
        commit::rows(&committed.files, &committed.numstat)
    });
    rows.extend(
        pending
            .iter()
            .flat_map(|pending| commit::rows(&pending.files, &pending.numstat)),
    );
    tui::change_rows(&rows);

    let (generated, elapsed) = tui::timed_spinner("Generating pull request workflow", || {
        let context = Context {
            current_branch: &branch,
            new_branch: create_branch,
            base: Some(&base),
            user_prompt: None,
            committed: committed.as_ref(),
            pending: pending.as_ref(),
            issues: &issues,
        };
        generation::generate(provider, &context)
    })?;
    let pull_request = generated
        .pull_request
        .expect("generation requires pull request");
    let head = generated.branch.as_deref().unwrap_or(&branch);

    tui::step("Workflow generated", elapsed);
    if let Some(generated_branch) = &generated.branch {
        tui::section("Branch");
        tui::message(generated_branch);
    }
    if let Some(message) = &generated.commit {
        tui::section("Commit");
        tui::message(message);
    }
    tui::section("Title");
    tui::message(&pull_request.title);
    tui::section("Body");
    tui::block(&pull_request.body);

    let action = if create_branch {
        format!("Create {head}, commit, push, and create PR into {base}?")
    } else if pending.is_some() {
        format!("Commit, push {head}, and create PR into {base}?")
    } else {
        format!("Push {head} and create PR into {base}?")
    };
    if !tui::confirm(yes, &action)? {
        tui::aborted();
        return Ok(());
    }

    let upstream = if create_branch {
        tui::spinner("Creating branch", || git::run(&["checkout", "-b", head]))?;
        tui::success("Checked out", head);
        tui::rail();
        None
    } else {
        git::optional_upstream()
    };
    match &generated.commit {
        Some(message) => commit::finish(head, message, upstream)?,
        None => commit::push(head, upstream, "Pushing branch")?,
    }

    tui::rail();
    let url = tui::spinner("Creating pull request", || {
        github::create_pr(&base, head, &pull_request.title, &pull_request.body, draft)
    })?;
    tui::success("Created PR", &url);

    Ok(())
}

fn committed_changes(base_ref: &str) -> anyhow::Result<Option<Committed>> {
    let range = format!("{base_ref}...HEAD");
    let diff = |flag: &str| git::run(&["diff", flag, &range]);
    let files = diff("--name-status")?;
    let commits = git::run(&["log", "--oneline", &format!("{base_ref}..HEAD")])?;
    if files.is_empty() && commits.is_empty() {
        return Ok(None);
    }

    Ok(Some(Committed {
        stat: diff("--stat")?,
        numstat: diff("--numstat")?,
        diff: diff("--unified=3")?.chars().take(MAX_DIFF_CHARS).collect(),
        files,
        commits,
    }))
}
