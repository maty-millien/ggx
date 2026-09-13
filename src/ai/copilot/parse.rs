use serde_json::Value;

pub(super) struct Session {
    pub(super) token: String,
    pub(super) proxy: String,
}

pub(super) fn cached_session(contents: &str, now: u64) -> Option<Session> {
    let value: Value = serde_json::from_str(contents).ok()?;
    if value.get("expires_at")?.as_u64()? <= now + 60 {
        return None;
    }

    Some(Session {
        token: value.get("token")?.as_str()?.to_string(),
        proxy: value.get("endpoints")?.get("proxy")?.as_str()?.to_string(),
    })
}

pub(super) fn oauth_token_from_apps(apps: &Value) -> Option<String> {
    apps.as_object()?
        .iter()
        .find(|(host, _)| host.starts_with("github.com"))
        .and_then(|(_, app)| app.get("oauth_token")?.as_str())
        .map(str::to_string)
}

pub(super) fn stream_text(raw: &str) -> String {
    raw.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter_map(|data| serde_json::from_str::<Value>(data).ok())
        .filter_map(|event| {
            event
                .get("choices")?
                .get(0)?
                .get("text")?
                .as_str()
                .map(str::to_string)
        })
        .collect()
}

pub(super) fn first_json(text: &str) -> String {
    serde_json::Deserializer::from_str(text)
        .into_iter::<Value>()
        .next()
        .and_then(Result::ok)
        .map_or_else(|| text.to_string(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{cached_session, first_json, oauth_token_from_apps, stream_text};
    use serde_json::json;

    #[test]
    fn cached_session_requires_unexpired_token_and_proxy() {
        let response = |expires_at: u64| {
            json!({"token": "tid", "expires_at": expires_at, "endpoints": {"proxy": "https://proxy"}})
                .to_string()
        };

        let session = cached_session(&response(1_000), 900).unwrap();
        assert_eq!(session.token, "tid");
        assert_eq!(session.proxy, "https://proxy");

        assert!(cached_session(&response(960), 900).is_none());
        assert!(cached_session(r#"{"token":"tid","expires_at":1000}"#, 900).is_none());
        assert!(cached_session("not json", 0).is_none());
    }

    #[test]
    fn oauth_token_from_apps_reads_github_host_entry() {
        let apps = json!({
            "ghe.example.com:app": {"oauth_token": "enterprise"},
            "github.com:Iv1.abc": {"oauth_token": "gho_token"},
        });

        assert_eq!(oauth_token_from_apps(&apps), Some("gho_token".to_string()));
        assert_eq!(oauth_token_from_apps(&json!({})), None);
        assert_eq!(oauth_token_from_apps(&json!({"github.com": {}})), None);
        assert_eq!(oauth_token_from_apps(&json!([])), None);
    }

    #[test]
    fn stream_text_joins_completion_chunks() {
        let raw = [
            r#"data: {"choices":[{"text":"\"feat/"}]}"#,
            "data: not json",
            r#"data: {"choices":[]}"#,
            r#"data: {"choices":[{"text":"x\"}"}]}"#,
            "data: [DONE]",
        ]
        .join("\n");

        assert_eq!(stream_text(&raw), r#""feat/x"}"#);
    }

    #[test]
    fn first_json_keeps_first_object_and_falls_back_to_raw_text() {
        assert_eq!(
            first_json(r#"{"branch":"feat/a"} trailing {"other":1}"#),
            r#"{"branch":"feat/a"}"#
        );
        assert_eq!(first_json("{\"branch\":"), "{\"branch\":");
    }
}
