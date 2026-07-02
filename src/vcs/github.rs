use std::process::{Command, Stdio};

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

pub fn pull_request(target: Option<&str>) -> anyhow::Result<PullRequest> {
    let mut args = vec![
        "pr",
        "view",
        "--json",
        "number,title,url,headRefName,baseRefName,mergeStateStatus,reviewDecision,statusCheckRollup",
    ];
    if let Some(target) = target {
        args.insert(2, target);
    }

    let output = run(&args)?;
    let value: serde_json::Value = serde_json::from_str(&output)?;

    Ok(PullRequest {
        number: json_string(&value, "number"),
        title: json_string(&value, "title"),
        url: json_string(&value, "url"),
        head: json_string(&value, "headRefName"),
        base: json_string(&value, "baseRefName"),
        merge_state: json_string(&value, "mergeStateStatus"),
        review_decision: json_string(&value, "reviewDecision"),
    })
}

pub fn merge(target: Option<&str>, keep_branch: bool, admin: bool) -> anyhow::Result<String> {
    merge_with_strategy(target, "--merge", keep_branch, admin)
}

pub fn squash(target: Option<&str>, keep_branch: bool, admin: bool) -> anyhow::Result<String> {
    merge_with_strategy(target, "--squash", keep_branch, admin)
}

fn merge_with_strategy(
    target: Option<&str>,
    strategy: &str,
    keep_branch: bool,
    admin: bool,
) -> anyhow::Result<String> {
    let mut args = vec!["pr", "merge"];
    if let Some(target) = target {
        args.push(target);
    }
    args.push(strategy);
    if !keep_branch {
        args.push("--delete-branch");
    }
    if admin {
        args.push("--admin");
    }

    Ok(run(&args)?.trim().to_string())
}

fn run(args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("gh")
        .args(args)
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| anyhow::anyhow!("failed to run gh: {}", error))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    anyhow::bail!("gh {} failed: {}", args.join(" "), stderr);
}

fn json_string(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .map(|value| match value {
            serde_json::Value::String(value) => value.clone(),
            serde_json::Value::Number(value) => value.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::json_string;
    use serde_json::json;

    #[test]
    fn json_string_reads_strings_and_numbers() {
        let value = json!({ "title": "Hello", "number": 42 });

        assert_eq!(json_string(&value, "title"), "Hello");
        assert_eq!(json_string(&value, "number"), "42");
    }

    #[test]
    fn json_string_defaults_for_missing_or_non_scalar_values() {
        let value = json!({ "labels": ["bug"], "closed": false });

        assert_eq!(json_string(&value, "missing"), "");
        assert_eq!(json_string(&value, "labels"), "");
        assert_eq!(json_string(&value, "closed"), "");
    }
}
