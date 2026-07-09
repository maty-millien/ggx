use super::generate::{AiProvider, direct_response_prompt};
use anyhow::Context;
use serde_json::Value;
use std::io;
use std::process::{Command, Stdio};

const INSTALL_AUTH_MESSAGE: &str = "Install OpenCode CLI, then authenticate with OpenCode Zen.";
const MODEL: &str = "opencode/north-mini-code-free";
const VARIANT: &str = "none";
const AGENT: &str = "title";

pub struct OpenCodeProvider;

impl AiProvider for OpenCodeProvider {
    fn generate(&self, prompt: &str) -> anyhow::Result<String> {
        let output = opencode_command(prompt).output().map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                anyhow::anyhow!("OpenCode CLI not found. {}", INSTALL_AUTH_MESSAGE)
            } else {
                anyhow::anyhow!("failed to run OpenCode CLI: {}", error)
            }
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if !output.status.success() {
            let stdout_message =
                parse_error_events(&stdout).or_else(|| parse_text_events(&stdout).ok());
            let message = if !stderr.is_empty() {
                stderr
            } else if let Some(stdout_message) = stdout_message {
                stdout_message
            } else {
                output.status.to_string()
            };
            anyhow::bail!("OpenCode CLI failed: {}.", message);
        }

        parse_text_events(&stdout)
    }
}

fn opencode_command(prompt: &str) -> Command {
    let mut command = Command::new("opencode");
    command
        .args([
            "run",
            "--format",
            "json",
            "--model",
            MODEL,
            "--variant",
            VARIANT,
            "--agent",
            AGENT,
        ])
        .arg(direct_response_prompt(prompt))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command
}

fn parse_error_events(output: &str) -> Option<String> {
    output.lines().rev().find_map(|line| {
        let value: Value = serde_json::from_str(line).ok()?;
        if value["type"] != "error" {
            return None;
        }

        value["error"]
            .as_str()
            .map(str::to_string)
            .or_else(|| Some(value["error"].to_string()))
            .filter(|message| !message.trim().is_empty())
    })
}

fn parse_text_events(output: &str) -> anyhow::Result<String> {
    let mut messages = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let value: Value =
            serde_json::from_str(line).context("failed to parse OpenCode CLI JSON output")?;
        if value["type"] == "text"
            && value["part"]["type"] == "text"
            && let Some(text) = value["part"]["text"].as_str()
            && !text.trim().is_empty()
        {
            messages.push(text.to_string());
        }
    }

    let message = messages.join("\n").trim().to_string();
    if message.is_empty() {
        anyhow::bail!("OpenCode CLI did not return a response.");
    }

    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::{AGENT, MODEL, VARIANT, opencode_command, parse_error_events, parse_text_events};
    use crate::ai::generate::direct_response_prompt;
    use std::ffi::OsStr;

    #[test]
    fn builds_non_interactive_opencode_command() {
        let command = opencode_command("hello");
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let expected = [
            "run",
            "--format",
            "json",
            "--model",
            MODEL,
            "--variant",
            VARIANT,
            "--agent",
            AGENT,
        ]
        .into_iter()
        .map(String::from)
        .chain([direct_response_prompt("hello")])
        .collect::<Vec<_>>();

        assert_eq!(command.get_program(), OsStr::new("opencode"));
        assert_eq!(args, expected);
    }

    #[test]
    fn parses_one_text_event() {
        let output =
            r#"{"type":"text","part":{"type":"text","text":"feat(ai): add opencode provider"}}"#;

        assert_eq!(
            parse_text_events(output).unwrap(),
            "feat(ai): add opencode provider"
        );
    }

    #[test]
    fn parses_multiple_text_events() {
        let output = r#"{"type":"step_start","part":{"type":"step-start"}}
{"type":"text","part":{"type":"text","text":"first"}}
{"type":"text","part":{"type":"text","text":"second"}}"#;

        assert_eq!(parse_text_events(output).unwrap(), "first\nsecond");
    }

    #[test]
    fn errors_when_no_text_events_are_returned() {
        let output = r#"{"type":"step_start","part":{"type":"step-start"}}"#;

        assert!(parse_text_events(output).is_err());
    }

    #[test]
    fn errors_on_malformed_json() {
        assert!(parse_text_events("{bad json").is_err());
    }

    #[test]
    fn parses_error_events() {
        let output = r#"{"type":"text","part":{"type":"text","text":"before"}}
{"type":"error","error":"permission denied"}"#;

        assert_eq!(parse_error_events(output).unwrap(), "permission denied");
    }
}
