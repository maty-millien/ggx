pub struct PullRequest {
    pub title: String,
    pub body: String,
}

impl PullRequest {
    pub fn from_parts(title: &str, body: &str) -> anyhow::Result<Self> {
        let title = title.trim().to_string();
        let body = body.trim().to_string();

        if title.is_empty() || body.is_empty() {
            anyhow::bail!("Generated pull request title and body must not be empty.");
        }

        Ok(Self { title, body })
    }
}
