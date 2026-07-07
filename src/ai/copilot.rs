use super::generate::{AiProvider, direct_response_prompt};
use serde_json::Value;
use std::io;
use std::process::{Command, Stdio};

const INSTALL_AUTH_MESSAGE: &str = "Install GitHub Copilot CLI, then run `copilot login`.";

pub struct CopilotProvider;

impl AiProvider for CopilotProvider {
    fn generate(&self, prompt: &str) -> anyhow::Result<String> {
        let output = copilot_command(prompt).output().map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                anyhow::anyhow!("GitHub Copilot CLI not found. {}", INSTALL_AUTH_MESSAGE)
            } else {
                anyhow::anyhow!("failed to run GitHub Copilot CLI: {}", error)
            }
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if !output.status.success() {
            let message = if stderr.is_empty() {
                output.status.to_string()
            } else {
                stderr
            };
            anyhow::bail!(
                "GitHub Copilot CLI failed: {}. Run `copilot login` if authentication is required.",
                message
            );
        }

        parse_assistant_message(&stdout)
    }
}

fn copilot_command(prompt: &str) -> Command {
    let mut command = Command::new("copilot");
    command
        .args([
            "--no-color",
            "--no-auto-update",
            "--no-custom-instructions",
            "--disable-builtin-mcps",
            "--no-remote",
            "--no-remote-export",
            "--output-format",
            "json",
            "--stream",
            "off",
            "-p",
        ])
        .arg(direct_response_prompt(prompt))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command
}

fn parse_assistant_message(output: &str) -> anyhow::Result<String> {
    let mut message = None;

    for line in output.lines() {
        let value: Value = serde_json::from_str(line)?;
        if value["type"] == "assistant.message" {
            message = value["data"]["content"].as_str().map(str::to_string);
        }
    }

    message
        .map(|message| message.trim().to_string())
        .filter(|message| !message.is_empty())
        .ok_or_else(|| anyhow::anyhow!("GitHub Copilot CLI did not return a response."))
}

#[cfg(test)]
mod tests {
    use super::{copilot_command, parse_assistant_message};
    use crate::ai::generate::direct_response_prompt;
    use std::ffi::OsStr;

    #[test]
    fn builds_non_interactive_copilot_command() {
        let command = copilot_command("hello");
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let expected = [
            "--no-color",
            "--no-auto-update",
            "--no-custom-instructions",
            "--disable-builtin-mcps",
            "--no-remote",
            "--no-remote-export",
            "--output-format",
            "json",
            "--stream",
            "off",
            "-p",
        ]
        .into_iter()
        .map(String::from)
        .chain([direct_response_prompt("hello")])
        .collect::<Vec<_>>();

        assert_eq!(command.get_program(), OsStr::new("copilot"));
        assert_eq!(args, expected);
    }

    #[test]
    fn parses_assistant_message_from_jsonl_output() {
        let output = r#"{"type":"session.tools_updated","data":{}}
{"type":"assistant.message","data":{"content":"feat(ai): add copilot provider"}}
{"type":"result","exitCode":0}"#;

        assert_eq!(
            parse_assistant_message(output).unwrap(),
            "feat(ai): add copilot provider"
        );
    }

    #[test]
    fn parses_last_assistant_message_from_jsonl_output() {
        let output = r#"{"type":"assistant.message","data":{"content":"first"}}
{"type":"assistant.message","data":{"content":"second"}}"#;

        assert_eq!(parse_assistant_message(output).unwrap(), "second");
    }
}
