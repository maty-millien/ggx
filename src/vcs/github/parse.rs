use super::PullRequest;

pub(super) fn optional_pull_request_from_output(
    args: &[&str],
    success: bool,
    stdout: &str,
    stderr: &str,
) -> anyhow::Result<Option<PullRequest>> {
    if success {
        let value: serde_json::Value = serde_json::from_str(stdout)?;
        let value = match value {
            serde_json::Value::Array(values) => {
                let Some(value) = values.into_iter().next() else {
                    return Ok(None);
                };
                value
            }
            value => value,
        };

        return parse_pull_request(&value.to_string()).map(Some);
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

pub(super) fn parse_pull_request(output: &str) -> anyhow::Result<PullRequest> {
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

pub(super) fn json_string(value: &serde_json::Value, key: &str) -> String {
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
    fn optional_pull_request_reads_first_listed_pr() {
        let output = r#"[{
            "number": 42,
            "title": "Add fast fail",
            "url": "https://github.com/owner/repo/pull/42",
            "headRefName": "feature",
            "baseRefName": "main",
            "mergeStateStatus": "CLEAN",
            "reviewDecision": "APPROVED"
        }, {"number": 43}]"#;

        let pull_request = optional_pull_request_from_output(&["pr", "list"], true, output, "")
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
    fn optional_pull_request_reads_single_object_output() {
        let pull_request =
            optional_pull_request_from_output(&["pr", "list"], true, r#"{"number": 7}"#, "")
                .unwrap()
                .unwrap();

        assert_eq!(pull_request.number, "7");
        assert_eq!(pull_request.title, "");
    }

    #[test]
    fn optional_pull_request_returns_none_for_empty_list() {
        assert!(
            optional_pull_request_from_output(&["pr", "list"], true, "[]", "")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn optional_pull_request_rejects_invalid_json() {
        assert!(optional_pull_request_from_output(&["pr", "list"], true, "not json", "").is_err());
    }

    #[test]
    fn optional_pull_request_returns_none_when_gh_reports_no_pr() {
        let pull_request = optional_pull_request_from_output(
            &["pr", "view", "feature"],
            false,
            "",
            "No pull requests found for branch \"feature\"",
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
        let error = result.err().expect("expected gh failure").to_string();

        assert!(error.contains("gh pr view feature failed: HTTP 401: Bad credentials"));
    }
}
