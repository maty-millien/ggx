use super::codex::CodexProvider;

pub trait AiProvider {
    fn generate(&self, prompt: &str) -> anyhow::Result<String>;
}

pub fn generate(prompt: &str) -> anyhow::Result<String> {
    CodexProvider.generate(prompt)
}
