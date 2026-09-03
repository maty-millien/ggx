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

#[cfg(test)]
mod tests {
    use super::PullRequest;

    #[test]
    fn builds_title_and_body() {
        let pull_request = PullRequest::from_parts("Add feature", "## Summary\nBody").unwrap();

        assert_eq!(pull_request.title, "Add feature");
        assert_eq!(pull_request.body, "## Summary\nBody");
    }

    #[test]
    fn trims_output_title_and_body() {
        let pull_request = PullRequest::from_parts("  Add feature  ", "  Body  \n").unwrap();

        assert_eq!(pull_request.title, "Add feature");
        assert_eq!(pull_request.body, "Body");
    }

    #[test]
    fn rejects_empty_title() {
        assert!(PullRequest::from_parts("", "Body").is_err());
    }

    #[test]
    fn rejects_empty_body() {
        assert!(PullRequest::from_parts("Title", "  ").is_err());
    }
}
