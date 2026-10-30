use super::{Provider, curl, now_secs, sse_events};
use crate::config;
use anyhow::Context;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::path::Path;
use std::{env, fs};

const MODEL: &str = "gpt-6-luna";
const CHATGPT_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
const OPENAI_URL: &str = "https://api.openai.com/v1";
const INSTRUCTIONS: &str = "You are a git workflow assistant.";
const NOT_SIGNED_IN: &str =
    "Codex is not signed in or the token has expired. Run 'codex login' first.";

struct Endpoint {
    url: String,
    headers: Vec<String>,
    model: String,
    chat: bool,
    reasoning: bool,
}

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    let endpoint = endpoint()?;
    let body = if endpoint.chat {
        json!({
            "model": endpoint.model,
            "messages": [
                {"role": "system", "content": INSTRUCTIONS},
                {"role": "user", "content": prompt},
            ],
        })
    } else {
        let mut body = json!({
            "model": endpoint.model,
            "instructions": INSTRUCTIONS,
            "input": [{"role": "user", "content": [{"type": "input_text", "text": prompt}]}],
            "stream": true,
            "store": false,
        });
        if endpoint.reasoning {
            body["reasoning"] = json!({"effort": "none"});
        }
        body
    }
    .to_string();

    let mut args = vec!["-H", "Content-Type: application/json"];
    args.extend(
        endpoint
            .headers
            .iter()
            .flat_map(|header| ["-H", header.as_str()]),
    );
    args.extend(["--data-binary", &body, &endpoint.url]);
    let raw = curl(Provider::Codex, &args)?;

    if endpoint.chat {
        let response: Value =
            serde_json::from_str(&raw).context("Codex returned an unexpected response")?;
        return Ok(response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string());
    }
    Ok(sse_events(&raw)
        .filter(|event| event["type"] == "response.output_text.delta")
        .filter_map(|event| event["delta"].as_str().map(str::to_owned))
        .collect())
}

pub fn validate() -> anyhow::Result<()> {
    endpoint().map(drop)
}

fn endpoint() -> anyhow::Result<Endpoint> {
    let dir = config::dir("CODEX_HOME", ".codex").context(NOT_SIGNED_IN)?;
    let path = dir.join("config.toml");
    let settings: toml::Table = match fs::read_to_string(&path) {
        Ok(contents) => contents
            .parse()
            .with_context(|| format!("Invalid Codex config at {}.", path.display()))?,
        Err(_) => toml::Table::new(),
    };
    let profile = settings
        .get("profile")
        .and_then(toml::Value::as_str)
        .and_then(|name| settings.get("profiles")?.get(name));
    let setting = |key: &str| {
        profile
            .and_then(|profile| profile.get(key))
            .or_else(|| settings.get(key))
            .and_then(toml::Value::as_str)
    };

    let provider = setting("model_provider").unwrap_or("openai");
    if provider == "openai" {
        return openai(&dir);
    }

    let table = settings
        .get("model_providers")
        .and_then(|providers| providers.get(provider))
        .with_context(|| {
            format!(
                "Codex provider '{provider}' is not defined in {}.",
                path.display()
            )
        })?;
    let field = |key: &str| table.get(key).and_then(toml::Value::as_str);
    let base_url = field("base_url")
        .with_context(|| format!("Codex provider '{provider}' has no base_url."))?;
    let mut headers: Vec<String> = table
        .get("http_headers")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flatten()
        .filter_map(|(name, value)| Some(format!("{name}: {}", value.as_str()?)))
        .collect();
    let key = match field("env_key") {
        Some(var) => Some(env_var(var).with_context(|| {
            format!("Codex provider '{provider}' needs the {var} environment variable.")
        })?),
        None => field("experimental_bearer_token").map(str::to_string),
    };
    headers.extend(key.map(|key| format!("Authorization: Bearer {key}")));
    let chat = field("wire_api") == Some("chat");

    Ok(Endpoint {
        url: format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            if chat {
                "chat/completions"
            } else {
                "responses"
            }
        ),
        headers,
        model: setting("model").unwrap_or(MODEL).to_string(),
        chat,
        reasoning: false,
    })
}

fn openai(dir: &Path) -> anyhow::Result<Endpoint> {
    let auth: Value = fs::read_to_string(dir.join("auth.json"))
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default();

    if let Some((access_token, account_id)) = chatgpt_tokens(&auth) {
        return Ok(Endpoint {
            url: CHATGPT_URL.to_string(),
            headers: vec![
                format!("Authorization: Bearer {access_token}"),
                format!("chatgpt-account-id: {account_id}"),
                "OpenAI-Beta: responses=experimental".to_string(),
                "originator: codex_cli_rs".to_string(),
            ],
            model: MODEL.to_string(),
            chat: false,
            reasoning: true,
        });
    }

    let key = auth["OPENAI_API_KEY"]
        .as_str()
        .filter(|key| !key.is_empty())
        .map(str::to_string)
        .or_else(|| env_var("OPENAI_API_KEY"))
        .context(NOT_SIGNED_IN)?;
    let base_url = env_var("OPENAI_BASE_URL").unwrap_or_else(|| OPENAI_URL.to_string());
    Ok(Endpoint {
        url: format!("{}/responses", base_url.trim_end_matches('/')),
        headers: vec![format!("Authorization: Bearer {key}")],
        model: MODEL.to_string(),
        chat: false,
        reasoning: true,
    })
}

fn chatgpt_tokens(auth: &Value) -> Option<(String, String)> {
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

fn env_var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}
