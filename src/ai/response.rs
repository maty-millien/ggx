const DIRECT_RESPONSE_INSTRUCTIONS: &str = r#"Do not invoke tools.
Do not inspect files.
Do not run commands.
Return only the requested text and nothing else.

"#;

pub(crate) fn direct_response_prompt(prompt: &str) -> String {
    format!("{}{}", DIRECT_RESPONSE_INSTRUCTIONS, prompt)
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
    use super::{direct_response_prompt, strip_markdown_fence};

    #[test]
    fn prefixes_generation_prompt_with_direct_response_instructions() {
        assert_eq!(
            direct_response_prompt("name a branch"),
            "Do not invoke tools.\nDo not inspect files.\nDo not run commands.\nReturn only the requested text and nothing else.\n\nname a branch"
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
}
