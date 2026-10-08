use crate::ai::{self, Provider};
use crate::commands::{commit::Changes, pr::Committed};
use crate::{git, github};
use anyhow::{Context as _, bail, ensure};
use serde_json::Value;

const BRANCH_TYPES: &[&str] = &["feat", "fix", "refactor", "docs", "test", "chore"];
const COMMIT_TYPES: &[&str] = &[
    "feat", "fix", "refactor", "docs", "test", "chore", "build", "ci",
];

/// What the model sees. It names a branch when `new_branch` is set, writes a
/// commit message when there are `pending` changes, and writes a pull
/// request when there is a `base`.
#[derive(Default)]
pub struct Context<'a> {
    pub current_branch: &'a str,
    pub new_branch: bool,
    pub base: Option<&'a str>,
    pub user_prompt: Option<&'a str>,
    pub committed: Option<&'a Committed>,
    pub pending: Option<&'a Changes>,
    pub issues: &'a [github::Issue],
}

pub struct Output {
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub pull_request: Option<PullRequest>,
}

pub struct PullRequest {
    pub title: String,
    pub body: String,
}

/// Asks the model, and asks once more with the reason if the reply is rejected.
pub fn generate(provider: Provider, context: &Context) -> anyhow::Result<Output> {
    let attempt = |retry: Option<&str>| -> anyhow::Result<Output> {
        let output = parse(&ai::generate(provider, &render(context, retry))?, context)?;
        if let Some(branch) = &output.branch
            && git::branch_exists(branch)?
        {
            bail!("Branch '{branch}' already exists.");
        }
        Ok(output)
    };

    attempt(None).or_else(|error| attempt(Some(&error.to_string())))
}

fn parse(raw: &str, context: &Context) -> anyhow::Result<Output> {
    let value: Value = serde_json::from_str(raw)?;
    let field = |key: &str| {
        value[key]
            .as_str()
            .with_context(|| format!("Generated output must include {key}."))
    };

    let branch = if context.new_branch {
        Some(normalize_branch(field("branch")?)?)
    } else {
        None
    };

    let commit = if context.pending.is_some() {
        let message = field("commit")?.trim();
        validate_commit(message)?;
        Some(message.to_string())
    } else {
        None
    };

    let pull_request = if context.base.is_some() {
        let pull_request = value["pull_request"]
            .as_object()
            .context("Generated output must include pull_request.")?;
        let title = pull_request
            .get("title")
            .and_then(Value::as_str)
            .context("Generated pull request must include a title.")?
            .trim();
        let body = pull_request
            .get("body")
            .and_then(Value::as_str)
            .context("Generated pull request must include a body.")?
            .trim();
        ensure!(
            !title.is_empty() && !body.is_empty(),
            "Generated pull request title and body must not be empty."
        );
        Some(PullRequest {
            title: title.to_string(),
            body: body.to_string(),
        })
    } else {
        None
    };

    Ok(Output {
        branch,
        commit,
        pull_request,
    })
}

/// Takes the first line outside a code fence, lowercases it and drops
/// characters git or the type/slug format reject.
fn normalize_branch(raw: &str) -> anyhow::Result<String> {
    let line = raw
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("```"))
        .unwrap_or_default();
    let mut branch = String::new();
    for character in line.chars().map(|character| character.to_ascii_lowercase()) {
        if character.is_ascii_alphanumeric()
            || character == '/'
            || (character == '-' && !branch.ends_with('-'))
        {
            branch.push(character);
        }
    }

    let Some((prefix, slug)) = branch.split_once('/') else {
        bail!("Generated branch name must use type/slug format.");
    };
    ensure!(
        BRANCH_TYPES.contains(&prefix),
        "Generated branch name used unsupported type '{prefix}'."
    );
    ensure!(
        !slug.contains('/'),
        "Generated branch name must contain only one slash."
    );
    let slug = slug.trim_matches('-');
    ensure!(
        !slug.is_empty(),
        "Generated branch name must include a slug."
    );

    Ok(format!("{prefix}/{slug}"))
}

fn validate_commit(message: &str) -> anyhow::Result<()> {
    ensure!(
        !message.is_empty() && !message.contains(['\n', '\r']),
        "Commit message must be exactly one line."
    );
    // The message is trimmed, so a subject after ": " is never empty.
    let Some((kind, _)) = message.split_once(": ") else {
        bail!("Commit message must use 'type(scope): subject'.");
    };
    let Some((commit_type, scope)) = kind.split_once('(') else {
        bail!("Commit message must include a non-empty scope.");
    };
    ensure!(
        COMMIT_TYPES.contains(&commit_type),
        "Commit message type '{commit_type}' is not allowed."
    );
    let Some(scope) = scope.strip_suffix(')') else {
        bail!("Commit message scope must close before the colon.");
    };
    ensure!(
        !scope.trim().is_empty() && !scope.contains(['(', ')']),
        "Commit message scope cannot be empty."
    );

    Ok(())
}

fn render(context: &Context, retry: Option<&str>) -> String {
    let branch_instruction = if context.new_branch {
        format!(
            "Set branch to a concise name using type/short-kebab-name. Allowed types: {}.",
            BRANCH_TYPES.join(", ")
        )
    } else {
        "Set branch to null.".to_string()
    };
    let commit_instruction = if context.pending.is_some() {
        format!(
            "Set commit to one Conventional Commit line using type(scope): subject. Allowed types: {}.",
            COMMIT_TYPES.join(", ")
        )
    } else {
        "Set commit to null.".to_string()
    };
    let pull_request_instruction = if context.base.is_none() {
        "Set pull_request to null.".to_string()
    } else {
        let issues = if context.issues.is_empty() {
            "Do not mention or close any issue; none are provided."
        } else {
            "Include GitHub closing references only for the issues listed under Issues To Close."
        };
        format!(
            "Set pull_request to an object with title and body strings. The body must be GitHub-flavored Markdown with ## Summary and ## Changes headings. Do not add test plan, risk, or notes sections. {issues}"
        )
    };
    let retry = retry.map_or(String::new(), |error| {
        format!(
            "\n## Previous Attempt\n\nThe previous response was rejected: {error}\nReturn a corrected, fully regenerated object.\n"
        )
    });
    let base = context.base.unwrap_or("Not applicable");
    let user_prompt = section("User Prompt", context.user_prompt);
    let committed = context.committed.map_or(String::new(), |committed| {
        format!(
            "\n## Existing Commits\n\n````\n{}\n````\n\n## Committed Changed Files\n\n````\n{}\n````\n\n## Committed Diff Stat\n\n````\n{}\n````\n\n## Committed Diff\n\n````diff\n{}\n````\n",
            committed.commits, committed.files, committed.stat, committed.diff
        )
    });
    let pending = context.pending.map_or(String::new(), render_pending);
    let issues: String = context
        .issues
        .iter()
        .map(|issue| {
            format!(
                "\n### {}\n\nReference: {}\nNumber: {}\nURL: {}\n\n````\n{}\n````\n",
                issue.title, issue.reference, issue.number, issue.url, issue.body
            )
        })
        .collect();
    let issues = if issues.is_empty() {
        issues
    } else {
        format!("\n## Issues To Close\n{issues}")
    };

    format!(
        r#"## Instructions

Generate the requested git workflow content as one JSON object.
Return valid JSON only, with exactly this shape:
{{"branch": string|null, "commit": string|null, "pull_request": {{"title": string, "body": string}}|null}}
Do not use markdown fences around the JSON. Do not explain the response.
{branch_instruction}
{commit_instruction}
{pull_request_instruction}

Keep every generated field consistent with the others.

## Current Branch

````
{}
````

## Pull Request Base

````
{}
````{}{}{}{}{}"#,
        context.current_branch, base, user_prompt, committed, pending, issues, retry
    )
}

fn section(title: &str, value: Option<&str>) -> String {
    value.map_or(String::new(), |value| {
        format!("\n## {title}\n\n````\n{value}\n````\n")
    })
}

fn render_pending(pending: &Changes) -> String {
    let readme = section("README", pending.readme.as_deref());
    let notes = if pending.notes.is_empty() {
        String::new()
    } else {
        format!("\n## Notes\n\n{}\n", pending.notes.join("\n"))
    };

    format!(
        "\n## Pending Changed Files\n\n````\n{}\n````\n\n## Pending Diff Stat\n\n````\n{}\n````\n\n## Pending Numstat\n\n````\n{}\n````\n\n## Pending Diff Summary\n\n````\n{}\n````{}\n## Pending Diff\n\n````diff\n{}\n````{}",
        pending.files, pending.stat, pending.numstat, pending.summary, readme, pending.diff, notes
    )
}
