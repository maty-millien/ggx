mod parse;

use parse::{json_string, optional_pull_request_from_output, parse_pull_request};
use std::process::{Command, Stdio};

const PR_JSON_FIELDS: &str =
    "number,title,url,headRefName,baseRefName,mergeStateStatus,reviewDecision,statusCheckRollup";

pub struct Issue {
    pub number: String,
    pub title: String,
    pub body: String,
    pub url: String,
}

pub struct PullRequest {
    pub number: String,
    pub title: String,
    pub url: String,
    pub head: String,
    pub base: String,
    pub merge_state: String,
    pub review_decision: String,
}

pub fn issue(reference: &str) -> anyhow::Result<Issue> {
    let output = run(&[
        "issue",
        "view",
        reference,
        "--json",
        "number,title,body,url",
    ])?;
    let value: serde_json::Value = serde_json::from_str(&output)?;

    Ok(Issue {
        number: json_string(&value, "number"),
        title: json_string(&value, "title"),
        body: json_string(&value, "body"),
        url: json_string(&value, "url"),
    })
}

pub fn create_pr(
    base: &str,
    branch: &str,
    title: &str,
    body: &str,
    draft: bool,
) -> anyhow::Result<String> {
    let mut args = vec![
        "pr", "create", "--base", base, "--head", branch, "--title", title, "--body", body,
    ];

    if draft {
        args.push("--draft");
    }

    Ok(run(&args)?.trim().to_string())
}

pub fn pull_request() -> anyhow::Result<PullRequest> {
    let output = run(&["pr", "view", "--json", PR_JSON_FIELDS])?;
    parse_pull_request(&output)
}

pub fn open_pull_request(branch: &str) -> anyhow::Result<Option<PullRequest>> {
    let args = vec![
        "pr",
        "list",
        "--head",
        branch,
        "--state",
        "open",
        "--limit",
        "1",
        "--json",
        PR_JSON_FIELDS,
    ];
    let output =
        run_output(&args).map_err(|error| anyhow::anyhow!("failed to run gh: {}", error))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    optional_pull_request_from_output(&args, output.status.success(), &stdout, &stderr)
}

pub fn merge(keep_branch: bool, admin: bool) -> anyhow::Result<String> {
    merge_with_strategy("--merge", keep_branch, admin)
}

pub fn squash(keep_branch: bool, admin: bool) -> anyhow::Result<String> {
    merge_with_strategy("--squash", keep_branch, admin)
}

fn merge_with_strategy(strategy: &str, keep_branch: bool, admin: bool) -> anyhow::Result<String> {
    let mut args = vec!["pr", "merge", strategy];
    if !keep_branch {
        args.push("--delete-branch");
    }
    if admin {
        args.push("--admin");
    }

    Ok(run(&args)?.trim().to_string())
}

fn run(args: &[&str]) -> anyhow::Result<String> {
    let output =
        run_output(args).map_err(|error| anyhow::anyhow!("failed to run gh: {}", error))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    anyhow::bail!("gh {} failed: {}", args.join(" "), stderr);
}

fn run_output(args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("gh")
        .args(args)
        .stderr(Stdio::piped())
        .output()
}
