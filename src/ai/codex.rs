use super::{Provider, curl, now_secs, sse_events};
use crate::config;
use anyhow::Context;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::fs;

const MODEL: &str = "gpt-6-luna";
const RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
const INSTRUCTIONS: &str = "You are a git workflow assistant.";
const NOT_SIGNED_IN: &str =
    "Codex is not signed in or the token has expired. Run 'codex login' first.";

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    let (access_token, account_id) = credentials().context(NOT_SIGNED_IN)?;
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

pub fn validate() -> anyhow::Result<()> {
    credentials().map(drop).context(NOT_SIGNED_IN)
}

fn credentials() -> Option<(String, String)> {
    let contents =
        fs::read_to_string(config::dir("CODEX_HOME", ".codex")?.join("auth.json")).ok()?;
    let auth: Value = serde_json::from_str(&contents).ok()?;
    let tokens = &auth["tokens"];
    let access_token = tokens["access_token"].as_str()?;
    let claims = URL_SAFE_NO_PAD
        .decode(access_token.split('.').nth(1)?)
        .ok()?;
    let claims: Value = serde_json::from_slice(&claims).ok()?;
    if claims["exp"].as_u64()? <= now_secs() + 60 {
        return None;
    }

    Some((
        access_token.to_string(),
        tokens["account_id"].as_str()?.to_string(),
    ))
}
