mod parse;

use super::direct_response_prompt;
use anyhow::Context;
use parse::{message_text, oauth_access_token};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const API_MODEL: &str = "claude-haiku-4-5";
const API_URL: &str = "https://api.anthropic.com";
const MAX_TOKENS: u32 = 4096;
const KEYCHAIN_SERVICE: &str = "Claude Code-credentials";
const NOT_SIGNED_IN: &str =
    "Claude is not signed in or the token has expired. Run 'claude login' first.";
// The API only accepts Claude CLI OAuth tokens when the system prompt opens
// with the CLI's own identity line.
const INSTRUCTIONS: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    // The CLI spends seconds on startup, so ggx talks to the Messages API
    // directly with the credentials the CLI itself uses and never falls back
    // to the CLI. ANTHROPIC_BASE_URL and ANTHROPIC_AUTH_TOKEN are honoured so
    // that a local proxy configured for the CLI also routes ggx.
    let base_url = env_var("ANTHROPIC_BASE_URL").unwrap_or_else(|| API_URL.to_string());
    let body = json!({
        "model": API_MODEL,
        "max_tokens": MAX_TOKENS,
        "system": INSTRUCTIONS,
        "messages": [{"role": "user", "content": direct_response_prompt(prompt)}],
    });

    let mut command = Command::new("curl");
    command.args(["-sS", "--fail-with-body", "--max-time", "60"]);
    command.args(["-H", "anthropic-version: 2023-06-01"]);
    command.args(["-H", "Content-Type: application/json"]);
    for header in auth_headers().ok_or_else(|| anyhow::anyhow!(NOT_SIGNED_IN))? {
        command.args(["-H", &header]);
    }
    let output = command
        .args(["--data-binary", &body.to_string()])
        .arg(format!("{base_url}/v1/messages"))
        .output()
        .context("Could not start curl. Install it before using the Claude provider.")?;
    if !output.status.success() {
        anyhow::bail!(
            "Claude request failed: {} {}",
            String::from_utf8_lossy(&output.stderr).trim(),
            String::from_utf8_lossy(&output.stdout).trim()
        );
    }

    let response: Value =
        serde_json::from_slice(&output.stdout).context("Claude returned an unexpected response")?;
    let text = message_text(&response);
    if text.is_empty() {
        anyhow::bail!("Claude returned an empty response");
    }
    Ok(super::strip_markdown_fence(text.trim()).to_string())
}

fn auth_headers() -> Option<Vec<String>> {
    if let Some(token) = env_var("ANTHROPIC_AUTH_TOKEN") {
        return Some(vec![format!("Authorization: Bearer {token}")]);
    }

    let token = oauth_access_token(&credentials()?, now_millis())?;
    Some(vec![
        format!("Authorization: Bearer {token}"),
        "anthropic-beta: oauth-2025-04-20".to_string(),
    ])
}

fn credentials() -> Option<String> {
    if let Ok(contents) = fs::read_to_string(config_directory()?.join(".credentials.json")) {
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

fn config_directory() -> Option<PathBuf> {
    env_var("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".claude")))
}

fn env_var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}
