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
