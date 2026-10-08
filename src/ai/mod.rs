mod claude;
mod codex;
mod copilot;

use anyhow::{Context, bail};
use serde_json::Value;
use std::process::{Command, Stdio};
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

    /// The configuration value and the provider's CLI binary.
    pub fn name(self) -> String {
        self.label().to_lowercase()
    }
}

const DIRECT_RESPONSE_INSTRUCTIONS: &str = r#"Do not invoke tools.
Do not inspect files.
Do not run commands.
Return only the requested text and nothing else.

"#;

pub fn generate(provider: Provider, prompt: &str) -> anyhow::Result<String> {
    let direct = || format!("{DIRECT_RESPONSE_INSTRUCTIONS}{prompt}");
    let text = match provider {
        Provider::Codex => codex::generate(&direct())?,
        Provider::Claude => claude::generate(&direct())?,
        Provider::Copilot => copilot::generate(prompt)?,
    };

    let text = text.trim();
    if text.is_empty() {
        bail!("{} returned an empty response", provider.label());
    }
    Ok(strip_markdown_fence(text).to_string())
}

pub fn validate(provider: Provider) -> anyhow::Result<()> {
    if provider == Provider::Copilot {
        return copilot::validate();
    }

    let name = provider.name();
    let output = Command::new(&name)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .output()
        .with_context(|| {
            format!(
                "Could not start {} CLI (`{name}`). Install it before running `ggx setup`.",
                provider.label()
            )
        })?;

    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if detail.is_empty() {
            output.status.to_string()
        } else {
            detail
        };
        bail!("{} CLI is not usable: {detail}", provider.label());
    }

    Ok(())
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

/// JSON payloads of a server-sent event stream.
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
