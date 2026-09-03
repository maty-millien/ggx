use crate::vcs::git;

pub struct Context {
    pub prompt: Option<String>,
    pub branch: String,
    pub has_changes: bool,
}

impl Context {
    pub fn collect(prompt: Option<String>) -> anyhow::Result<Self> {
        let prompt = prompt
            .map(|prompt| prompt.trim().to_string())
            .filter(|prompt| !prompt.is_empty());

        let has_changes = git::has_changes()?;

        if prompt.is_none() && !has_changes {
            anyhow::bail!("No staged or unstaged changes found.");
        }

        Ok(Self {
            prompt,
            branch: git::run(&["rev-parse", "--abbrev-ref", "HEAD"])?
                .trim()
                .to_string(),
            has_changes,
        })
    }

    pub fn has_changes(&self) -> bool {
        self.has_changes
    }
}
