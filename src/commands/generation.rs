use crate::ai;
use crate::commands::{branch, commit, pr};
use crate::vcs::git;
use serde_json::Value;

pub struct Context {
    pub current_branch: String,
    pub base: Option<String>,
    pub user_prompt: Option<String>,
    pub commits: String,
    pub committed_files: String,
    pub committed_stat: String,
    pub committed_diff: String,
    pub pending: Option<PendingChanges>,
    pub issues: Vec<Issue>,
}

pub struct PendingChanges {
    pub files: String,
    pub stat: String,
    pub numstat: String,
    pub summary: String,
    pub readme: Option<String>,
    pub diff: String,
    pub notes: Vec<&'static str>,
}

impl From<&commit::context::Context> for PendingChanges {
    fn from(context: &commit::context::Context) -> Self {
        let mut notes = Vec::new();
        if context.diff_truncated {
            notes.push("Diff exceeded context budget.");
        }
        if context.diff_file_truncated {
            notes.push("One or more file diffs were truncated.");
        }
        if context.readme_truncated {
            notes.push("README was truncated.");
        }

        Self {
            files: context.files.clone(),
            stat: context.stat.clone(),
            numstat: context.numstat.clone(),
            summary: context.summary.clone(),
            readme: context.readme.clone(),
            diff: context.diff.clone(),
            notes,
        }
    }
}

pub struct Issue {
    pub reference: String,
    pub number: String,
    pub title: String,
    pub body: String,
    pub url: String,
}

#[derive(Clone, Copy)]
pub struct Request {
    pub branch: bool,
    pub commit: bool,
    pub pull_request: bool,
}

pub struct Output {
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub pull_request: Option<pr::validation::PullRequest>,
}

pub fn generate(context: &Context, request: Request) -> anyhow::Result<Output> {
    generate_with(context, request, ai::generate, git::branch_exists)
}

fn generate_with<G, B>(
    context: &Context,
    request: Request,
    mut generate_text: G,
    mut branch_exists: B,
) -> anyhow::Result<Output>
where
    G: FnMut(&str) -> anyhow::Result<String>,
    B: FnMut(&str) -> anyhow::Result<bool>,
{
    let mut retry = None;

    for attempt in 0..2 {
        let prompt = render(context, request, retry.as_deref());
        let result = generate_text(&prompt)
            .and_then(|raw| parse(&raw, request))
            .and_then(|output| validate_branch(output, request, &mut branch_exists));

        match result {
            Ok(output) => return Ok(output),
            Err(error) if attempt == 0 => retry = Some(error.to_string()),
            Err(error) => return Err(error),
        }
    }

    unreachable!("generation attempts are bounded")
}

fn validate_branch<B>(
    output: Output,
    request: Request,
    branch_exists: &mut B,
) -> anyhow::Result<Output>
where
    B: FnMut(&str) -> anyhow::Result<bool>,
{
    if request.branch {
        let branch = output.branch.as_deref().expect("parser requires branch");
        if branch_exists(branch)? {
            anyhow::bail!("Branch '{}' already exists.", branch);
        }
    }

    Ok(output)
}

fn parse(raw: &str, request: Request) -> anyhow::Result<Output> {
    let value: Value = serde_json::from_str(raw.trim())?;

    let branch = if request.branch {
        let raw = required_string(&value, "branch")?;
        Some(branch::validation::normalize(raw)?)
    } else {
        None
    };

    let commit = if request.commit {
        let message = required_string(&value, "commit")?.trim().to_string();
        commit::validation::validate(&message)?;
        Some(message)
    } else {
        None
    };

    let pull_request = if request.pull_request {
        let pull_request = value
            .get("pull_request")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow::anyhow!("Generated output must include pull_request."))?;
        let title = pull_request
            .get("title")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Generated pull request must include a title."))?;
        let body = pull_request
            .get("body")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Generated pull request must include a body."))?;

        Some(pr::validation::PullRequest::from_parts(title, body)?)
    } else {
        None
    };

    Ok(Output {
        branch,
        commit,
        pull_request,
    })
}

fn required_string<'a>(value: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Generated output must include {}.", key))
}

fn render(context: &Context, request: Request, retry: Option<&str>) -> String {
    let branch_instruction = if request.branch {
        "Set branch to a concise name using type/short-kebab-name. Allowed types: feat, fix, refactor, docs, test, chore."
    } else {
        "Set branch to null."
    };
    let commit_instruction = if request.commit {
        "Set commit to one Conventional Commit line using type(scope): subject. Allowed types: feat, fix, refactor, docs, test, chore, build, ci."
    } else {
        "Set commit to null."
    };
    let pull_request_instruction = if request.pull_request {
        "Set pull_request to an object with title and body strings. The body must be GitHub-flavored Markdown with ## Summary and ## Changes headings. Do not add test plan, risk, or notes sections. Include GitHub closing references for provided issues."
    } else {
        "Set pull_request to null."
    };
    let retry = retry.map_or(String::new(), |error| {
        format!(
            "\n## Previous Attempt\n\nThe previous response was rejected: {}\nReturn a corrected, fully regenerated object.\n",
            error
        )
    });
    let base = context.base.as_deref().unwrap_or("Not applicable");
    let user_prompt = optional_section("User Prompt", context.user_prompt.as_deref());
    let committed = if context.commits.is_empty() && context.committed_files.is_empty() {
        String::new()
    } else {
        format!(
            "\n## Existing Commits\n\n````\n{}\n````\n\n## Committed Changed Files\n\n````\n{}\n````\n\n## Committed Diff Stat\n\n````\n{}\n````\n\n## Committed Diff\n\n````diff\n{}\n````\n",
            context.commits,
            context.committed_files,
            context.committed_stat,
            context.committed_diff
        )
    };
    let pending = context
        .pending
        .as_ref()
        .map_or(String::new(), render_pending);
    let issues = if context.issues.is_empty() {
        String::new()
    } else {
        format!("\n## Issues To Close\n{}", render_issues(&context.issues))
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

fn optional_section(title: &str, value: Option<&str>) -> String {
    value.map_or(String::new(), |value| {
        format!("\n## {}\n\n````\n{}\n````\n", title, value)
    })
}

fn render_pending(pending: &PendingChanges) -> String {
    let readme = optional_section("README", pending.readme.as_deref());
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

fn render_issues(issues: &[Issue]) -> String {
    issues
        .iter()
        .map(|issue| {
            format!(
                "\n### {}\n\nReference: {}\nNumber: {}\nURL: {}\n\n````\n{}\n````\n",
                issue.title, issue.reference, issue.number, issue.url, issue.body
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::{Context, Issue, PendingChanges, Request, generate_with, parse, render};
    use std::cell::Cell;

    fn context() -> Context {
        Context {
            current_branch: "main".to_string(),
            base: Some("dev".to_string()),
            user_prompt: Some("add retries".to_string()),
            commits: "abc feat(api): add endpoint".to_string(),
            committed_files: "M\tsrc/api.rs".to_string(),
            committed_stat: "1 file changed".to_string(),
            committed_diff: "diff --git a/src/api.rs b/src/api.rs".to_string(),
            pending: Some(PendingChanges {
                files: "M\tsrc/main.rs".to_string(),
                stat: "1 file changed".to_string(),
                numstat: "1\t0\tsrc/main.rs".to_string(),
                summary: String::new(),
                readme: Some("# App".to_string()),
                diff: "diff --git a/src/main.rs b/src/main.rs".to_string(),
                notes: Vec::new(),
            }),
            issues: vec![Issue {
                reference: "#12".to_string(),
                number: "12".to_string(),
                title: "Retry requests".to_string(),
                body: "Retries are missing.".to_string(),
                url: "https://example.com/12".to_string(),
            }],
        }
    }

    #[test]
    fn renders_all_requested_artifacts_and_context() {
        let prompt = render(
            &context(),
            Request {
                branch: true,
                commit: true,
                pull_request: true,
            },
            None,
        );

        assert!(prompt.contains("Set branch to a concise name"));
        assert!(prompt.contains("Set commit to one Conventional Commit"));
        assert!(prompt.contains("## Pull Request Base\n\n````\ndev"));
        assert!(prompt.contains("## Existing Commits"));
        assert!(prompt.contains("## Pending Changed Files"));
        assert!(prompt.contains("Reference: #12"));
    }

    #[test]
    fn parses_requested_json_fields() {
        let output = parse(
            r###"{"branch":"feat/add-retries","commit":"feat(api): add retries","pull_request":{"title":"Add retries","body":"## Summary\nAdd retries.\n\n## Changes\n- Retry requests."}}"###,
            Request {
                branch: true,
                commit: true,
                pull_request: true,
            },
        )
        .unwrap();

        assert_eq!(output.branch.as_deref(), Some("feat/add-retries"));
        assert_eq!(output.commit.as_deref(), Some("feat(api): add retries"));
        assert_eq!(output.pull_request.unwrap().title, "Add retries");
    }

    #[test]
    fn ignores_unrequested_null_fields() {
        let output = parse(
            r#"{"branch":null,"commit":"fix(cli): handle error","pull_request":null}"#,
            Request {
                branch: false,
                commit: true,
                pull_request: false,
            },
        )
        .unwrap();

        assert!(output.branch.is_none());
        assert!(output.pull_request.is_none());
    }

    #[test]
    fn retries_once_after_invalid_output() {
        let calls = Cell::new(0);
        let output = generate_with(
            &context(),
            Request {
                branch: true,
                commit: false,
                pull_request: false,
            },
            |_| {
                calls.set(calls.get() + 1);
                if calls.get() == 1 {
                    Ok("not json".to_string())
                } else {
                    Ok(r#"{"branch":"feat/retry","commit":null,"pull_request":null}"#.to_string())
                }
            },
            |_| Ok(false),
        )
        .unwrap();

        assert_eq!(calls.get(), 2);
        assert_eq!(output.branch.as_deref(), Some("feat/retry"));
    }

    #[test]
    fn retries_once_when_branch_exists() {
        let calls = Cell::new(0);
        let output = generate_with(
            &context(),
            Request {
                branch: true,
                commit: false,
                pull_request: false,
            },
            |_| {
                calls.set(calls.get() + 1);
                let branch = if calls.get() == 1 {
                    "feat/existing"
                } else {
                    "feat/replacement"
                };
                Ok(format!(
                    r#"{{"branch":"{}","commit":null,"pull_request":null}}"#,
                    branch
                ))
            },
            |branch| Ok(branch == "feat/existing"),
        )
        .unwrap();

        assert_eq!(calls.get(), 2);
        assert_eq!(output.branch.as_deref(), Some("feat/replacement"));
    }

    #[test]
    fn stops_after_two_invalid_outputs() {
        let calls = Cell::new(0);
        let result = generate_with(
            &context(),
            Request {
                branch: true,
                commit: false,
                pull_request: false,
            },
            |_| {
                calls.set(calls.get() + 1);
                Ok("invalid".to_string())
            },
            |_| Ok(false),
        );

        assert!(result.is_err());
        assert_eq!(calls.get(), 2);
    }
}
