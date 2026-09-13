mod stream;

use super::{Provider, direct_response_prompt, run};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use stream::stream_text;

const MODEL: &str = "gpt-5.6-luna";
const REASONING_EFFORT: &str = "model_reasoning_effort=\"none\"";
const RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
const INSTRUCTIONS: &str = "You are a git workflow assistant.";

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    // The CLI spends seconds on startup; talk to its backend directly and only
    // fall back to the CLI when the cached token is missing, expired, or rejected.
    // The CLI refreshes and persists the token, so the next call is fast again.
    match direct(prompt) {
        Some(response) => Ok(response),
        None => run(Provider::Codex, codex_command(), prompt),
    }
}

fn direct(prompt: &str) -> Option<String> {
    let auth: Value = serde_json::from_str(&fs::read_to_string(auth_path()?).ok()?).ok()?;
    let tokens = auth.get("tokens")?;
    let access_token = tokens.get("access_token")?.as_str()?;
    let account_id = tokens.get("account_id")?.as_str()?;
    let body = json!({
        "model": MODEL,
        "instructions": INSTRUCTIONS,
        "input": [{"role": "user", "content": [{"type": "input_text", "text": direct_response_prompt(prompt)}]}],
        "stream": true,
        "store": false,
        "reasoning": {"effort": "none"},
    });

    let output = Command::new("curl")
        .args(["-sS", "--fail", "--max-time", "60", "--no-buffer"])
        .args(["-H", &format!("Authorization: Bearer {access_token}")])
        .args(["-H", &format!("chatgpt-account-id: {account_id}")])
        .args(["-H", "OpenAI-Beta: responses=experimental"])
        .args(["-H", "originator: codex_cli_rs"])
        .args(["-H", "Content-Type: application/json"])
        .args(["--data-binary", &body.to_string(), RESPONSES_URL])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = stream_text(&String::from_utf8_lossy(&output.stdout));
    (!text.is_empty()).then(|| super::strip_markdown_fence(text.trim()).to_string())
}

fn auth_path() -> Option<PathBuf> {
    env::var_os("CODEX_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .map(|base| base.join("auth.json"))
}

fn codex_command() -> Command {
    let mut command = Command::new("codex");
    command.args([
        "exec",
        "--ephemeral",
        "--ignore-user-config",
        "--sandbox",
        "read-only",
        "--disable",
        "apps",
        "--disable",
        "browser_use",
        "--disable",
        "computer_use",
        "--disable",
        "goals",
        "--disable",
        "image_generation",
        "--disable",
        "multi_agent",
        "--disable",
        "shell_tool",
        "--disable",
        "workspace_dependencies",
        "--model",
        MODEL,
        "-c",
        REASONING_EFFORT,
        "-",
    ]);

    command
}

#[cfg(test)]
mod tests {
    use super::{MODEL, codex_command};
    use std::ffi::OsStr;
    #[test]
    fn codex_command_runs_read_only_exec_from_stdin() {
        let command = codex_command();
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(command.get_program(), OsStr::new("codex"));
        assert_eq!(args[0], "exec");
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--sandbox", "read-only"])
        );
        assert!(args.windows(2).any(|pair| pair == ["--model", MODEL]));
        assert_eq!(args.last().map(String::as_str), Some("-"));
    }
}
