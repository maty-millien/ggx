mod claude;
mod codex;
mod copilot;

use anyhow::{Context, bail};
use serde_json::Value;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum Provider {
    Codex,
    Claude,
    Copilot,
}

impl Provider {
    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude",
            Self::Copilot => "Copilot",
        }
    }

    pub fn name(self) -> String {
        self.label().to_lowercase()
    }
}

pub fn generate(provider: Provider, prompt: &str) -> anyhow::Result<String> {
    let text = match provider {
        Provider::Codex => codex::generate(prompt)?,
        Provider::Claude => claude::generate(prompt)?,
        Provider::Copilot => copilot::generate(prompt)?,
    };

    let text = text.trim();
    if text.is_empty() {
        bail!("{} returned an empty response", provider.label());
    }
    Ok(strip_markdown_fence(text).to_string())
}

pub fn validate(provider: Provider) -> anyhow::Result<()> {
    match provider {
        Provider::Codex => codex::validate(),
        Provider::Claude => claude::validate(),
        Provider::Copilot => copilot::validate(),
    }
}

fn curl(provider: Provider, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("curl")
        .args(["-sS", "--fail-with-body", "--max-time", "60"])
        .args(args)
        .output()
        .with_context(|| {
            format!(
                "Could not start curl. Install it before using the {} provider.",
                provider.label()
            )
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        bail!(
            "{} request failed: {} {}",
            provider.label(),
            String::from_utf8_lossy(&output.stderr).trim(),
            stdout.trim()
        );
    }

    Ok(stdout)
}

fn sse_events(raw: &str) -> impl Iterator<Item = Value> + '_ {
    raw.lines()
        .filter_map(|line| serde_json::from_str(line.strip_prefix("data: ")?).ok())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

fn strip_markdown_fence(response: &str) -> &str {
    let Some((opening, rest)) = response.split_once('\n') else {
        return response;
    };
    if opening != "```" && !opening.eq_ignore_ascii_case("```json") {
        return response;
    }

    rest.strip_suffix("\n```").map_or(response, str::trim)
}
