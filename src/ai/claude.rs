use super::{Provider, curl, now_secs};
use crate::config;
use anyhow::Context;
use serde_json::{Value, json};
use std::process::Command;
use std::{env, fs};

const API_MODEL: &str = "claude-haiku-5-5";
const API_URL: &str = "https://api.anthropic.com";
const MAX_TOKENS: u32 = 4096;
const KEYCHAIN_SERVICE: &str = "Claude Code-credentials";
const NOT_SIGNED_IN: &str =
    "Claude is not signed in or the token has expired. Run 'claude login' first.";
const INSTRUCTIONS: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    let base_url = env_var("ANTHROPIC_BASE_URL").unwrap_or_else(|| API_URL.to_string());
    let url = format!("{base_url}/v1/messages");
    let body = json!({
        "model": API_MODEL,
        "max_tokens": MAX_TOKENS,
        "system": INSTRUCTIONS,
        "messages": [{"role": "user", "content": prompt}],
    })
    .to_string();
    let headers = auth_headers().context(NOT_SIGNED_IN)?;

    let mut args = vec![
        "-H",
        "anthropic-version: 2023-06-01",
        "-H",
        "Content-Type: application/json",
    ];
    args.extend(headers.iter().flat_map(|header| ["-H", header.as_str()]));
    args.extend(["--data-binary", &body, &url]);

    let response: Value = serde_json::from_str(&curl(Provider::Claude, &args)?)
        .context("Claude returned an unexpected response")?;
    Ok(response["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|block| block["type"] == "text")
        .filter_map(|block| block["text"].as_str())
        .collect())
}

fn auth_headers() -> Option<Vec<String>> {
    if let Some(token) = env_var("ANTHROPIC_AUTH_TOKEN") {
        return Some(vec![format!("Authorization: Bearer {token}")]);
    }

    let credentials: Value = serde_json::from_str(&credentials()?).ok()?;
    let oauth = &credentials["claudeAiOauth"];
    if oauth["expiresAt"].as_u64()? <= (now_secs() + 60) * 1000 {
        return None;
    }

    let token = oauth["accessToken"]
        .as_str()
        .filter(|token| !token.is_empty())?;
    Some(vec![
        format!("Authorization: Bearer {token}"),
        "anthropic-beta: oauth-2025-04-20".to_string(),
    ])
}

fn credentials() -> Option<String> {
    if let Some(contents) = config::dir("CLAUDE_CONFIG_DIR", ".claude")
        .and_then(|dir| fs::read_to_string(dir.join(".credentials.json")).ok())
    {
        return Some(contents);
    }
    if !cfg!(target_os = "macos") {
        return None;
    }

    let output = Command::new("security")
        .args(["find-generic-password", "-s", KEYCHAIN_SERVICE, "-w"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn env_var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}
