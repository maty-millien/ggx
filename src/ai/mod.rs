mod claude;
mod codex;
mod copilot;
mod provider;
mod response;

use anyhow::Context;
use std::process::{Command, Stdio};

pub use provider::Provider;
pub(crate) use response::{direct_response_prompt, strip_markdown_fence};

pub fn generate(provider: Provider, prompt: &str) -> anyhow::Result<String> {
    match provider {
        Provider::Codex => codex::generate(prompt),
        Provider::Claude => claude::generate(prompt),
        Provider::Copilot => copilot::generate(prompt),
    }
}

pub fn validate(provider: Provider) -> anyhow::Result<()> {
    if provider == Provider::Copilot {
        return copilot::validate();
    }

    let output = Command::new(provider.as_str())
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| {
            format!(
                "Could not start {} CLI (`{}`). Install it before running `ggx setup`.",
                provider,
                provider.as_str()
            )
        })?;

    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if detail.is_empty() {
            anyhow::bail!("{} CLI is not usable: {}", provider, output.status);
        }
        anyhow::bail!("{} CLI is not usable: {}", provider, detail);
    }

    Ok(())
}
