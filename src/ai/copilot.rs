use anyhow::Context;
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";
const COMPLETIONS_PATH: &str = "/v1/engines/copilot-codex/completions";
const TOKEN_CACHE: &str = "copilot-token.json";
const JSON_SEED: &str = "{\"branch\":";
const MAX_TOKENS: u32 = 1024;
const NOT_SIGNED_IN: &str = "Copilot is not signed in. Sign in with the GitHub Copilot CLI or an editor with Copilot first.";
const HEADERS: [&str; 4] = [
    "Editor-Version: vscode/1.104.0",
    "Editor-Plugin-Version: copilot/1.370.0",
    "User-Agent: GithubCopilot/1.370.0",
    "OpenAI-Intent: copilot-ghost",
];

struct Session {
    token: String,
    proxy: String,
}

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    let session = session()?;
    let body = json!({
        "prompt": format!("{prompt}\n\n## Response\n\n{JSON_SEED}"),
        "suffix": "",
        "max_tokens": MAX_TOKENS,
        "temperature": 0,
        "n": 1,
        "stream": true,
        "extra": {"language": "json"},
    });
    let raw = curl(
        &format!("{}{}", session.proxy, COMPLETIONS_PATH),
        &format!("Authorization: Bearer {}", session.token),
        Some(&body.to_string()),
    )?;

    Ok(first_json(&format!("{JSON_SEED}{}", stream_text(&raw))))
}

pub fn validate() -> anyhow::Result<()> {
    session().map(|_| ())
}

fn session() -> anyhow::Result<Session> {
    let cache = crate::config::sibling(TOKEN_CACHE)?;
    if let Some(session) = fs::read_to_string(&cache)
        .ok()
        .and_then(|contents| cached_session(&contents, now()))
    {
        return Ok(session);
    }

    let oauth_token = oauth_token()?;
    let raw = curl(
        TOKEN_URL,
        &format!("Authorization: token {oauth_token}"),
        None,
    )
    .context("Could not obtain a Copilot token")?;
    let session = cached_session(&raw, now())
        .ok_or_else(|| anyhow::anyhow!("Copilot returned an unexpected token response"))?;

    if let Some(parent) = cache.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&cache, raw)?;
    Ok(session)
}

fn cached_session(contents: &str, now: u64) -> Option<Session> {
    let value: Value = serde_json::from_str(contents).ok()?;
    if value.get("expires_at")?.as_u64()? <= now + 60 {
        return None;
    }

    Some(Session {
        token: value.get("token")?.as_str()?.to_string(),
        proxy: value.get("endpoints")?.get("proxy")?.as_str()?.to_string(),
    })
}

fn oauth_token() -> anyhow::Result<String> {
    let directory = copilot_config_directory().ok_or_else(|| anyhow::anyhow!(NOT_SIGNED_IN))?;
    let apps = fs::read_to_string(directory.join("apps.json"))
        .ok()
        .and_then(|contents| serde_json::from_str::<Value>(&contents).ok())
        .and_then(|apps| oauth_token_from_apps(&apps));
    let hosts = || {
        fs::read_to_string(directory.join("hosts.json"))
            .ok()
            .and_then(|contents| serde_json::from_str::<Value>(&contents).ok())
            .and_then(|hosts| oauth_token_from_apps(&hosts))
    };

    apps.or_else(hosts)
        .ok_or_else(|| anyhow::anyhow!(NOT_SIGNED_IN))
}

fn oauth_token_from_apps(apps: &Value) -> Option<String> {
    apps.as_object()?
        .iter()
        .find(|(host, _)| host.starts_with("github.com"))
        .and_then(|(_, app)| app.get("oauth_token")?.as_str())
        .map(str::to_string)
}

fn copilot_config_directory() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|base| base.join("github-copilot"))
}

fn curl(url: &str, authorization: &str, body: Option<&str>) -> anyhow::Result<String> {
    let mut command = Command::new("curl");
    command.args(["-sS", "--fail-with-body", "--max-time", "60"]);
    for header in HEADERS {
        command.args(["-H", header]);
    }
    command.args(["-H", authorization, "-H", "Accept: application/json"]);
    if let Some(body) = body {
        command.args([
            "-H",
            "Content-Type: application/json",
            "--data-binary",
            body,
        ]);
    }
    command.arg(url);

    let output = command
        .output()
        .context("Could not start curl. Install it before using the Copilot provider.")?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        anyhow::bail!("Copilot request failed: {} {}", stderr, stdout.trim());
    }

    Ok(stdout)
}

fn stream_text(raw: &str) -> String {
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

fn first_json(text: &str) -> String {
    serde_json::Deserializer::from_str(text)
        .into_iter::<Value>()
        .next()
        .and_then(Result::ok)
        .map_or_else(|| text.to_string(), |value| value.to_string())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}
