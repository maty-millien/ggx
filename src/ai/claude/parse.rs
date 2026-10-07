use serde_json::Value;

pub(super) fn message_text(response: &Value) -> String {
    response
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|block| block.get("text")?.as_str())
        .collect()
}

pub(super) fn oauth_access_token(credentials: &str, now_millis: u64) -> Option<String> {
    let value: Value = serde_json::from_str(credentials).ok()?;
    let oauth = value.get("claudeAiOauth")?;
    if oauth.get("expiresAt")?.as_u64()? <= now_millis + 60_000 {
        return None;
    }

    let token = oauth.get("accessToken")?.as_str()?;
    (!token.is_empty()).then(|| token.to_string())
}
