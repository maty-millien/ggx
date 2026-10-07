use super::PullRequest;

pub(super) fn first_pull_request(output: &str) -> anyhow::Result<Option<PullRequest>> {
    let values: Vec<serde_json::Value> = serde_json::from_str(output)?;
    values
        .first()
        .map(|value| parse_pull_request(&value.to_string()))
        .transpose()
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
