use super::{Provider, curl, sse_events};
use crate::config;
use anyhow::{Context, bail};
use serde_json::{Value, json};
use std::fs;

const MODEL: &str = "gpt-6-luna";
const RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
const INSTRUCTIONS: &str = "You are a git workflow assistant.";
const NOT_SIGNED_IN: &str =
    "Codex is not signed in. Run 'codex login' to refresh the token in auth.json.";

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    // The CLI spends seconds on startup, so ggx talks to its backend directly
    // with the token the CLI stores and never falls back to the CLI.
    let auth: Value = config::dir("CODEX_HOME", ".codex")
        .and_then(|dir| fs::read_to_string(dir.join("auth.json")).ok())
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .context(NOT_SIGNED_IN)?;
    let tokens = &auth["tokens"];
    let (Some(access_token), Some(account_id)) = (
        tokens["access_token"].as_str(),
        tokens["account_id"].as_str(),
    ) else {
        bail!(NOT_SIGNED_IN);
    };
    let body = json!({
        "model": MODEL,
        "instructions": INSTRUCTIONS,
        "input": [{"role": "user", "content": [{"type": "input_text", "text": prompt}]}],
        "stream": true,
        "store": false,
        "reasoning": {"effort": "none"},
    })
    .to_string();

    let raw = curl(
        Provider::Codex,
        &[
            "-H",
            &format!("Authorization: Bearer {access_token}"),
            "-H",
            &format!("chatgpt-account-id: {account_id}"),
            "-H",
            "OpenAI-Beta: responses=experimental",
            "-H",
            "originator: codex_cli_rs",
            "-H",
            "Content-Type: application/json",
            "--data-binary",
            &body,
            RESPONSES_URL,
        ],
    )?;

    Ok(sse_events(&raw)
        .filter(|event| event["type"] == "response.output_text.delta")
        .filter_map(|event| event["delta"].as_str().map(str::to_owned))
        .collect())
}
