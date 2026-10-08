use serde_json::Value;

pub struct Issue {
    pub reference: String,
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
    let value: Value = serde_json::from_str(&output)?;

    Ok(Issue {
        reference: reference.to_string(),
        number: string(&value, "number"),
        title: string(&value, "title"),
        body: string(&value, "body"),
        url: string(&value, "url"),
    })
}

pub fn pull_request() -> anyhow::Result<PullRequest> {
    let output = run(&[
        "pr",
        "view",
        "--json",
        "number,title,url,headRefName,baseRefName,mergeStateStatus,reviewDecision",
    ])?;
    let value: Value = serde_json::from_str(&output)?;

    Ok(PullRequest {
        number: string(&value, "number"),
        title: string(&value, "title"),
        url: string(&value, "url"),
        head: string(&value, "headRefName"),
        base: string(&value, "baseRefName"),
        merge_state: string(&value, "mergeStateStatus"),
        review_decision: string(&value, "reviewDecision"),
    })
}

pub fn open_pull_request(branch: &str) -> anyhow::Result<Option<String>> {
    let output = run(&[
        "pr", "list", "--head", branch, "--state", "open", "--limit", "1", "--json", "url",
    ])?;
    let values: Vec<Value> = serde_json::from_str(&output)?;
    Ok(values.first().map(|value| string(value, "url")))
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
    run(&args)
}

pub fn merge(squash: bool, keep_branch: bool, admin: bool) -> anyhow::Result<String> {
    let mut args = vec!["pr", "merge", if squash { "--squash" } else { "--merge" }];
    if !keep_branch {
        args.push("--delete-branch");
    }
    if admin {
        args.push("--admin");
    }
    run(&args)
}

fn run(args: &[&str]) -> anyhow::Result<String> {
    crate::run("gh", args, &[])
}

fn string(value: &Value, key: &str) -> String {
    match &value[key] {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        _ => String::new(),
    }
}
