use super::{Provider, direct_response_prompt, run};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MODEL: &str = "haiku";
const API_MODEL: &str = "claude-haiku-4-5";
const API_URL: &str = "https://api.anthropic.com";
const MAX_TOKENS: u32 = 4096;
const KEYCHAIN_SERVICE: &str = "Claude Code-credentials";
// The API only accepts Claude CLI OAuth tokens when the system prompt opens
// with the CLI's own identity line.
const INSTRUCTIONS: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    // The CLI spends seconds on startup; talk to the Messages API directly with
    // the credentials the CLI itself uses and only fall back to the CLI when
    // they are missing, expired, or rejected. The CLI refreshes and persists
    // its OAuth token, so the next call is fast again.
    match direct(prompt) {
        Some(response) => Ok(response),
        None => run(Provider::Claude, claude_command(), prompt),
    }
}

fn direct(prompt: &str) -> Option<String> {
    let base_url = env_var("ANTHROPIC_BASE_URL").unwrap_or_else(|| API_URL.to_string());
    let body = json!({
        "model": API_MODEL,
        "max_tokens": MAX_TOKENS,
        "output_config": {"effort": "none"},
        "system": INSTRUCTIONS,
        "messages": [{"role": "user", "content": direct_response_prompt(prompt)}],
    });

    let mut command = Command::new("curl");
    command.args(["-sS", "--fail", "--max-time", "60"]);
    command.args(["-H", "anthropic-version: 2023-06-01"]);
    command.args(["-H", "Content-Type: application/json"]);
    for header in auth_headers()? {
        command.args(["-H", &header]);
    }
    let output = command
        .args(["--data-binary", &body.to_string()])
        .arg(format!("{base_url}/v1/messages"))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let response: Value = serde_json::from_slice(&output.stdout).ok()?;
    let text = message_text(&response);
    (!text.is_empty()).then(|| super::strip_markdown_fence(text.trim()).to_string())
}

fn message_text(response: &Value) -> String {
    response
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|block| block.get("text")?.as_str())
        .collect()
}

fn auth_headers() -> Option<Vec<String>> {
    if let Some(token) = env_var("ANTHROPIC_AUTH_TOKEN") {
        return Some(vec![format!("Authorization: Bearer {token}")]);
    }
    if let Some(key) = env_var("ANTHROPIC_API_KEY") {
        return Some(vec![format!("x-api-key: {key}")]);
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

fn oauth_access_token(credentials: &str, now_millis: u64) -> Option<String> {
    let value: Value = serde_json::from_str(credentials).ok()?;
    let oauth = value.get("claudeAiOauth")?;
    if oauth.get("expiresAt")?.as_u64()? <= now_millis + 60_000 {
        return None;
    }

    let token = oauth.get("accessToken")?.as_str()?;
    (!token.is_empty()).then(|| token.to_string())
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

fn claude_command() -> Command {
    let mut command = Command::new("claude");
    command.args([
        "--print",
        "--output-format",
        "text",
        "--safe-mode",
        "--tools",
        "",
        "--no-session-persistence",
        "--no-chrome",
        "--model",
        MODEL,
    ]);
    command
}

#[cfg(test)]
mod tests {
    use super::{MODEL, claude_command};
    use std::ffi::OsStr;

    #[test]
    fn builds_non_interactive_claude_command() {
        let command = claude_command();
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let expected = [
            "--print",
            "--output-format",
            "text",
            "--safe-mode",
            "--tools",
            "",
            "--no-session-persistence",
            "--no-chrome",
            "--model",
            MODEL,
        ]
        .map(String::from);

        assert_eq!(command.get_program(), OsStr::new("claude"));
        assert_eq!(args, expected);
    }
}
