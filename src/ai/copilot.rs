use super::{Provider, curl, now_secs, sse_events};
use crate::config;
use anyhow::Context;
use serde_json::{Value, json};
use std::fs;

const TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";
const COMPLETIONS_PATH: &str = "/v1/engines/copilot-codex/completions";
const TOKEN_CACHE: &str = "copilot-token.json";
const JSON_SEED: &str = "{\"branch\":";
const MAX_TOKENS: u32 = 1024;
const NOT_SIGNED_IN: &str = "Copilot is not signed in. Sign in with the GitHub Copilot CLI or an editor with Copilot first.";
const HEADERS: [&str; 5] = [
    "Editor-Version: vscode/1.104.0",
    "Editor-Plugin-Version: copilot/1.370.0",
    "User-Agent: GithubCopilot/1.370.0",
    "OpenAI-Intent: copilot-ghost",
    "Accept: application/json",
];

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    let (token, proxy) = session()?;
    let body = json!({
        "prompt": format!("{prompt}\n\n## Response\n\n{JSON_SEED}"),
        "suffix": "",
        "max_tokens": MAX_TOKENS,
        "temperature": 0,
        "n": 1,
        "stream": true,
        "extra": {"language": "json"},
    })
    .to_string();
    let raw = request(
        &format!("Authorization: Bearer {token}"),
        Some(&body),
        &format!("{proxy}{COMPLETIONS_PATH}"),
    )?;

    // The completion continues the seed, which may be followed by more text
    // than the JSON object, so keep only the first JSON value.
    let completion: String = sse_events(&raw)
        .filter_map(|event| event["choices"][0]["text"].as_str().map(str::to_owned))
        .collect();
    let text = format!("{JSON_SEED}{completion}");
    let first = serde_json::Deserializer::from_str(&text)
        .into_iter::<Value>()
        .next()
        .and_then(Result::ok);
    Ok(first.map_or(text, |value| value.to_string()))
}

pub fn validate() -> anyhow::Result<()> {
    session().map(drop)
}

/// A cached or freshly exchanged Copilot token and its API endpoint.
fn session() -> anyhow::Result<(String, String)> {
    let cache = config::sibling(TOKEN_CACHE)?;
    if let Some(session) = fs::read_to_string(&cache)
        .ok()
        .and_then(|contents| parse_session(&contents))
    {
        return Ok(session);
    }

    let authorization = format!("Authorization: token {}", oauth_token()?);
    let raw =
        request(&authorization, None, TOKEN_URL).context("Could not obtain a Copilot token")?;
    let session = parse_session(&raw).context("Copilot returned an unexpected token response")?;
    config::write(&cache, raw)?;
    Ok(session)
}

fn parse_session(contents: &str) -> Option<(String, String)> {
    let value: Value = serde_json::from_str(contents).ok()?;
    if value["expires_at"].as_u64()? <= now_secs() + 60 {
        return None;
    }

    Some((
        value["token"].as_str()?.to_string(),
        value["endpoints"]["proxy"].as_str()?.to_string(),
    ))
}

/// The GitHub login saved by the Copilot CLI or an editor plugin.
fn oauth_token() -> anyhow::Result<String> {
    let directory = config::dir("XDG_CONFIG_HOME", ".config")
        .context(NOT_SIGNED_IN)?
        .join("github-copilot");

    ["apps.json", "hosts.json"]
        .into_iter()
        .find_map(|file| {
            let apps: Value =
                serde_json::from_str(&fs::read_to_string(directory.join(file)).ok()?).ok()?;
            apps.as_object()?
                .iter()
                .find(|(host, _)| host.starts_with("github.com"))
                .and_then(|(_, app)| app["oauth_token"].as_str())
                .map(str::to_string)
        })
        .context(NOT_SIGNED_IN)
}

fn request(authorization: &str, body: Option<&str>, url: &str) -> anyhow::Result<String> {
    let mut args: Vec<&str> = HEADERS
        .iter()
        .chain([&authorization])
        .flat_map(|header| ["-H", header])
        .collect();
    if let Some(body) = body {
        args.extend([
            "-H",
            "Content-Type: application/json",
            "--data-binary",
            body,
        ]);
    }
    args.push(url);
    curl(Provider::Copilot, &args)
}
