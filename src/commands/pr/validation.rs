pub struct PullRequest {
    pub title: String,
    pub body: String,
}

impl PullRequest {
    pub fn parse(output: &str) -> anyhow::Result<Self> {
        let output = output.trim();
        let Some((title, body)) = output.split_once("\n\n") else {
            anyhow::bail!("Generated pull request must include a title, blank line, and body.");
        };

        let title = title.trim().to_string();
        let body = body.trim().to_string();

        if title.is_empty() || body.is_empty() {
            anyhow::bail!("Generated pull request title and body must not be empty.");
        }

        Ok(Self { title, body })
    }
}

#[cfg(test)]
mod tests {
    use super::PullRequest;

    #[test]
    fn parses_title_and_body() {
        let pull_request = PullRequest::parse("Add feature\n\n## Summary\nBody").unwrap();

        assert_eq!(pull_request.title, "Add feature");
        assert_eq!(pull_request.body, "## Summary\nBody");
    }

    #[test]
    fn trims_output_title_and_body() {
        let pull_request = PullRequest::parse("  Add feature  \n\n  Body  \n").unwrap();

        assert_eq!(pull_request.title, "Add feature");
        assert_eq!(pull_request.body, "Body");
    }

    #[test]
    fn rejects_missing_blank_line() {
        assert!(PullRequest::parse("Add feature\nBody").is_err());
    }

    #[test]
    fn rejects_empty_title() {
        assert!(PullRequest::parse("\n\nBody").is_err());
    }

    #[test]
    fn rejects_empty_body() {
        assert!(PullRequest::parse("Title\n\n  ").is_err());
    }
}
