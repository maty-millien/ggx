use super::opencode::OpenCodeProvider;

const DIRECT_RESPONSE_INSTRUCTIONS: &str = r#"Do not invoke tools.
Do not inspect files.
Do not run commands.
Return only the requested text and nothing else.

"#;

pub trait AiProvider {
    fn generate(&self, prompt: &str) -> anyhow::Result<String>;
}

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    OpenCodeProvider.generate(prompt)
}

pub(super) fn direct_response_prompt(prompt: &str) -> String {
    format!("{}{}", DIRECT_RESPONSE_INSTRUCTIONS, prompt)
}

#[cfg(test)]
mod tests {
    use super::direct_response_prompt;

    #[test]
    fn direct_response_prompt_prefixes_generation_prompt() {
        assert_eq!(
            direct_response_prompt("name a branch"),
            "Do not invoke tools.\nDo not inspect files.\nDo not run commands.\nReturn only the requested text and nothing else.\n\nname a branch"
        );
    }
}
