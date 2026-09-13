use super::Provider;

const DIRECT_RESPONSE_INSTRUCTIONS: &str = r#"Do not invoke tools.
Do not inspect files.
Do not run commands.
Return only the requested text and nothing else.

"#;

pub(crate) fn direct_response_prompt(prompt: &str) -> String {
    format!("{}{}", DIRECT_RESPONSE_INSTRUCTIONS, prompt)
}

pub(super) fn response(
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

pub(crate) fn strip_markdown_fence(response: &str) -> &str {
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
    use super::{Provider, direct_response_prompt, response, strip_markdown_fence};

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
    fn strip_markdown_fence_handles_fence_variants() {
        assert_eq!(strip_markdown_fence("```\nplain\n```"), "plain");
        assert_eq!(strip_markdown_fence("```JSON\n{}\n```"), "{}");
        assert_eq!(strip_markdown_fence("no fence"), "no fence");
        assert_eq!(strip_markdown_fence("```rust\nfn\n```"), "```rust\nfn\n```");
        assert_eq!(strip_markdown_fence("```\nunclosed"), "```\nunclosed");
    }

    #[test]
    fn reports_provider_failure() {
        let error = response(Provider::Claude, false, b"", b"not authenticated").unwrap_err();
        assert_eq!(error.to_string(), "Claude CLI failed: not authenticated");

        let error = response(Provider::Codex, false, b"partial", b"").unwrap_err();
        assert_eq!(error.to_string(), "Codex CLI failed");
    }
}
