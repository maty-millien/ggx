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

#[cfg(test)]
mod tests {
    use super::{message_text, oauth_access_token};
    use serde_json::json;

    #[test]
    fn message_text_joins_text_blocks_only() {
        let response = json!({"content": [
            {"type": "text", "text": "{\"commit\""},
            {"type": "tool_use", "name": "bash"},
            {"type": "text", "text": ":null}"},
        ]});

        assert_eq!(message_text(&response), r#"{"commit":null}"#);
        assert_eq!(message_text(&json!({"error": "bad"})), "");
    }

    #[test]
    fn oauth_access_token_requires_unexpired_token() {
        let credentials = |token: &str, expires_at: u64| {
            json!({"claudeAiOauth": {"accessToken": token, "expiresAt": expires_at}}).to_string()
        };

        assert_eq!(
            oauth_access_token(&credentials("abc", 200_000), 100_000),
            Some("abc".to_string())
        );
        assert_eq!(
            oauth_access_token(&credentials("abc", 160_000), 100_000),
            None
        );
        assert_eq!(oauth_access_token(&credentials("", 200_000), 100_000), None);
        assert_eq!(oauth_access_token("{}", 0), None);
        assert_eq!(oauth_access_token("not json", 0), None);
    }
}
