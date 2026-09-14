mod stream;

use super::direct_response_prompt;
use anyhow::Context;
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use stream::stream_text;

const MODEL: &str = "gpt-5.6-luna";
const RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
const INSTRUCTIONS: &str = "You are a git workflow assistant.";
const NOT_SIGNED_IN: &str =
    "Codex is not signed in. Run 'codex login' to refresh the token in auth.json.";

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    // The CLI spends seconds on startup, so ggx talks to its backend directly
    // with the token the CLI stores and never falls back to the CLI.
    let auth_path = auth_path().ok_or_else(|| anyhow::anyhow!(NOT_SIGNED_IN))?;
    let auth: Value = fs::read_to_string(&auth_path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .ok_or_else(|| anyhow::anyhow!(NOT_SIGNED_IN))?;
    let tokens = auth.get("tokens");
    let access_token = tokens
        .and_then(|tokens| tokens.get("access_token")?.as_str())
        .ok_or_else(|| anyhow::anyhow!(NOT_SIGNED_IN))?;
    let account_id = tokens
        .and_then(|tokens| tokens.get("account_id")?.as_str())
        .ok_or_else(|| anyhow::anyhow!(NOT_SIGNED_IN))?;
    let body = json!({
        "model": MODEL,
        "instructions": INSTRUCTIONS,
        "input": [{"role": "user", "content": [{"type": "input_text", "text": direct_response_prompt(prompt)}]}],
        "stream": true,
        "store": false,
        "reasoning": {"effort": "none"},
    });

    let output = Command::new("curl")
        .args(["-sS", "--fail-with-body", "--max-time", "60", "--no-buffer"])
        .args(["-H", &format!("Authorization: Bearer {access_token}")])
        .args(["-H", &format!("chatgpt-account-id: {account_id}")])
        .args(["-H", "OpenAI-Beta: responses=experimental"])
        .args(["-H", "originator: codex_cli_rs"])
        .args(["-H", "Content-Type: application/json"])
        .args(["--data-binary", &body.to_string(), RESPONSES_URL])
        .output()
        .context("Could not start curl. Install it before using the Codex provider.")?;
    if !output.status.success() {
        anyhow::bail!(
            "Codex request failed: {} {}",
            String::from_utf8_lossy(&output.stderr).trim(),
            String::from_utf8_lossy(&output.stdout).trim()
        );
    }

    let text = stream_text(&String::from_utf8_lossy(&output.stdout));
    if text.is_empty() {
        anyhow::bail!("Codex returned an empty response");
    }
    Ok(super::strip_markdown_fence(text.trim()).to_string())
}

fn auth_path() -> Option<PathBuf> {
    env::var_os("CODEX_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .map(|base| base.join("auth.json"))
}
