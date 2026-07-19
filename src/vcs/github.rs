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

pub fn pull_request(target: Option<&str>) -> anyhow::Result<PullRequest> {
    let args = pr_view_args(target);

    let output = run(&args)?;
    parse_pull_request(&output)
}

pub fn open_pull_request(branch: &str) -> anyhow::Result<Option<PullRequest>> {
    let args = pr_view_args(Some(branch));
    let output =
        run_output(&args).map_err(|error| anyhow::anyhow!("failed to run gh: {}", error))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    optional_pull_request_from_output(&args, output.status.success(), &stdout, &stderr)
}

pub fn merge(keep_branch: bool, admin: bool) -> anyhow::Result<String> {
    merge_with_strategy(None, "--merge", keep_branch, admin)
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

fn pr_view_args(target: Option<&str>) -> Vec<&str> {
    let mut args = vec!["pr", "view", "--json", PR_JSON_FIELDS];
    if let Some(target) = target {
        args.insert(2, target);
    }

    args
}

fn optional_pull_request_from_output(
    args: &[&str],
    success: bool,
    stdout: &str,
    stderr: &str,
) -> anyhow::Result<Option<PullRequest>> {
    if success {
        return parse_pull_request(stdout).map(Some);
    }

    let stderr = stderr.trim();
    if stderr
        .to_ascii_lowercase()
        .contains("no pull requests found")
    {
        return Ok(None);
    }

    anyhow::bail!("gh {} failed: {}", args.join(" "), stderr);
}

fn parse_pull_request(output: &str) -> anyhow::Result<PullRequest> {
    let value: serde_json::Value = serde_json::from_str(output)?;

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
    use super::{json_string, optional_pull_request_from_output};
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

    #[test]
    fn optional_pull_request_reads_existing_pr() {
        let output = r#"{
            "number": 42,
            "title": "Add fast fail",
            "url": "https://github.com/owner/repo/pull/42",
            "headRefName": "feature",
            "baseRefName": "main",
            "mergeStateStatus": "CLEAN",
            "reviewDecision": "APPROVED"
        }"#;

        let pull_request =
            optional_pull_request_from_output(&["pr", "view", "feature"], true, output, "")
                .unwrap()
                .unwrap();

        assert_eq!(pull_request.number, "42");
        assert_eq!(pull_request.title, "Add fast fail");
        assert_eq!(pull_request.url, "https://github.com/owner/repo/pull/42");
        assert_eq!(pull_request.head, "feature");
        assert_eq!(pull_request.base, "main");
        assert_eq!(pull_request.merge_state, "CLEAN");
        assert_eq!(pull_request.review_decision, "APPROVED");
    }

    #[test]
    fn optional_pull_request_returns_none_when_no_pr_exists() {
        let pull_request = optional_pull_request_from_output(
            &["pr", "view", "feature"],
            false,
            "",
            "no pull requests found for branch \"feature\"",
        )
        .unwrap();

        assert!(pull_request.is_none());
    }

    #[test]
    fn optional_pull_request_preserves_other_gh_failures() {
        let result = optional_pull_request_from_output(
            &["pr", "view", "feature"],
            false,
            "",
            "HTTP 401: Bad credentials",
        );
        let Err(error) = result else {
            panic!("expected gh failure");
        };
        let error = error.to_string();

        assert!(error.contains("gh pr view feature failed: HTTP 401: Bad credentials"));
    }
}
