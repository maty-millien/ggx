mod claude;
mod codex;

use anyhow::Context;
use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};

const DIRECT_RESPONSE_INSTRUCTIONS: &str = r#"Do not invoke tools.
Do not inspect files.
Do not run commands.
Return only the requested text and nothing else.

"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Codex,
    Claude,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude",
        }
    }

    pub fn executable(self) -> &'static str {
        self.as_str()
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "codex" => Some(Self::Codex),
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

pub fn generate(provider: Provider, prompt: &str) -> anyhow::Result<String> {
    match provider {
        Provider::Codex => codex::generate(prompt),
        Provider::Claude => claude::generate(prompt),
    }
}

pub fn validate(provider: Provider) -> anyhow::Result<()> {
    let output = Command::new(provider.executable())
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| {
            format!(
                "Could not start {} CLI (`{}`). Install it before running `ggx setup`.",
                provider,
                provider.executable()
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

pub(crate) fn run(
    provider: Provider,
    mut command: Command,
    prompt: &str,
) -> anyhow::Result<String> {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().with_context(|| {
        format!(
            "Could not start {} CLI (`{}`). Run `ggx setup` after installing it.",
            provider,
            provider.executable()
        )
    })?;

    let stdin = child
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Failed to open {} CLI stdin", provider))?;
    stdin.write_all(direct_response_prompt(prompt).as_bytes())?;

    let output = child.wait_with_output()?;
    response(
        provider,
        output.status.success(),
        &output.stdout,
        &output.stderr,
    )
}

fn direct_response_prompt(prompt: &str) -> String {
    format!("{}{}", DIRECT_RESPONSE_INSTRUCTIONS, prompt)
}

fn response(
    provider: Provider,
    success: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> anyhow::Result<String> {
    let stdout = String::from_utf8_lossy(stdout);
    let stdout = strip_markdown_fence(stdout.trim()).to_string();
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();

    if !success {
        if stderr.is_empty() {
            anyhow::bail!("{} CLI failed", provider);
        }
        anyhow::bail!("{} CLI failed: {}", provider, stderr);
    }

    Ok(stdout)
}

fn strip_markdown_fence(response: &str) -> &str {
    let Some((opening, rest)) = response.split_once('\n') else {
        return response;
    };
    if opening != "```" && !opening.eq_ignore_ascii_case("```json") {
        return response;
    }

    rest.strip_suffix("\n```")
        .map(str::trim)
        .unwrap_or(response)
}

#[cfg(test)]
mod tests {
    use super::{Provider, direct_response_prompt, response};

    #[test]
    fn parses_provider_names() {
        assert_eq!(Provider::parse("codex"), Some(Provider::Codex));
        assert_eq!(Provider::parse("claude"), Some(Provider::Claude));
        assert_eq!(Provider::parse("other"), None);
    }

    #[test]
    fn prefixes_generation_prompt_with_direct_response_instructions() {
        assert_eq!(
            direct_response_prompt("name a branch"),
            "Do not invoke tools.\nDo not inspect files.\nDo not run commands.\nReturn only the requested text and nothing else.\n\nname a branch"
        );
    }

    #[test]
    fn trims_successful_response() {
        assert_eq!(
            response(Provider::Claude, true, b"  answer\n", b"").unwrap(),
            "answer"
        );
    }

    #[test]
    fn strips_json_markdown_fence_from_response() {
        assert_eq!(
            response(
                Provider::Claude,
                true,
                b"```json\n{\"commit\":\"fix(cli): handle error\"}\n```\n",
                b"",
            )
            .unwrap(),
            r#"{"commit":"fix(cli): handle error"}"#
        );
    }

    #[test]
    fn reports_provider_failure() {
        let error = response(Provider::Claude, false, b"", b"not authenticated").unwrap_err();

        assert_eq!(error.to_string(), "Claude CLI failed: not authenticated");
    }
}
