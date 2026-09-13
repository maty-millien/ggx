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

#[cfg(test)]
mod tests {
    use super::stream_text;

    #[test]
    fn stream_text_joins_output_deltas_only() {
        let raw = [
            "event: response.created",
            r#"data: {"type":"response.created"}"#,
            r#"data: {"type":"response.output_text.delta","delta":"{\"branch\""}"#,
            "data: not json",
            r#"data: {"type":"response.output_text.delta","delta":":null}"}"#,
            r#"data: {"type":"response.completed"}"#,
        ]
        .join("\n");

        assert_eq!(stream_text(&raw), r#"{"branch":null}"#);
        assert_eq!(stream_text(""), "");
    }
}
