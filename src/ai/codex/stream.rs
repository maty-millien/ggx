use serde_json::Value;

pub(super) fn stream_text(raw: &str) -> String {
    raw.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter_map(|data| serde_json::from_str::<Value>(data).ok())
        .filter(|event| {
            event.get("type").and_then(Value::as_str) == Some("response.output_text.delta")
        })
        .filter_map(|event| event.get("delta")?.as_str().map(str::to_string))
        .collect()
}
