use crate::commands::commit::context::Context as CommitContext;
use crate::commands::generation::Issue;
use crate::vcs::{git, github};

const MAX_DIFF_CHARS: usize = 16_000;
const MAX_ISSUE_BODY_CHARS: usize = 8_000;

pub struct Context {
    pub files: String,
    pub stat: String,
    pub numstat: String,
    pub commits: String,
    pub diff: String,
    pub pending: Option<CommitContext>,
    pub issues: Vec<Issue>,
}

impl Context {
    pub fn collect(branch: String, base: String, closes: Vec<String>) -> anyhow::Result<Self> {
        let base_ref = git::base_ref(&base)?;
        let (files, stat, numstat, commits, diff) = if branch == base {
            empty_committed_changes()
        } else {
            collect_committed_changes(&base_ref)?
        };
        let pending = if git::has_changes()? {
            Some(CommitContext::collect_for_branch(branch.clone())?)
        } else {
            None
        };

        if files.is_empty() && commits.is_empty() && pending.is_none() {
            anyhow::bail!("No changes found between {} and {}.", base, branch);
        }

        let issues = closes
            .into_iter()
            .map(collect_issue)
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(Self {
            files,
            stat,
            numstat,
            commits,
            diff,
            pending,
            issues,
        })
    }
}

fn empty_committed_changes() -> (String, String, String, String, String) {
    Default::default()
}

fn collect_committed_changes(
    base_ref: &str,
) -> anyhow::Result<(String, String, String, String, String)> {
    let range = format!("{}...HEAD", base_ref);
    let commit_range = format!("{}..HEAD", base_ref);
    let files = git::run(&["diff", "--name-status", &range])?
        .trim()
        .to_string();
    let stat = git::run(&["diff", "--stat", &range])?.trim().to_string();
    let numstat = git::run(&["diff", "--numstat", &range])?.trim().to_string();
    let commits = git::run(&["log", "--oneline", &commit_range])?
        .trim()
        .to_string();
    let diff = git::run(&["diff", "--unified=3", &range])?
        .trim()
        .to_string();
    let diff = diff.chars().take(MAX_DIFF_CHARS).collect();

    Ok((files, stat, numstat, commits, diff))
}

fn collect_issue(reference: String) -> anyhow::Result<Issue> {
    let issue = github::issue(&reference)?;

    Ok(Issue {
        reference,
        number: issue.number,
        title: issue.title,
        body: issue.body.chars().take(MAX_ISSUE_BODY_CHARS).collect(),
        url: issue.url,
    })
}
