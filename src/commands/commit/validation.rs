const ALLOWED_TYPES: &[&str] = &[
    "feat", "fix", "refactor", "docs", "test", "chore", "build", "ci",
];

pub fn validate(message: &str) -> anyhow::Result<()> {
    if message.is_empty() || message.contains('\n') || message.contains('\r') {
        anyhow::bail!("Commit message must be exactly one line.");
    }

    let Some((kind, subject)) = message.split_once(": ") else {
        anyhow::bail!("Commit message must use 'type(scope): subject'.");
    };

    if subject.trim().is_empty() {
        anyhow::bail!("Commit message subject cannot be empty.");
    }

    let Some((commit_type, scope)) = kind.split_once('(') else {
        anyhow::bail!("Commit message must include a non-empty scope.");
    };

    if !ALLOWED_TYPES.contains(&commit_type) {
        anyhow::bail!("Commit message type '{}' is not allowed.", commit_type);
    }

    let Some(scope) = scope.strip_suffix(')') else {
        anyhow::bail!("Commit message scope must close before the colon.");
    };

    if scope.trim().is_empty() || scope.contains('(') || scope.contains(')') {
        anyhow::bail!("Commit message scope cannot be empty.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn accepts_allowed_types_with_scope_and_subject() {
        for commit_type in [
            "feat", "fix", "refactor", "docs", "test", "chore", "build", "ci",
        ] {
            let message = format!("{}(scope): update behavior", commit_type);

            validate(&message).expect("message should be valid");
        }
    }

    #[test]
    fn rejects_missing_scope() {
        assert!(validate("feat: add thing").is_err());
    }

    #[test]
    fn rejects_empty_scope() {
        assert!(validate("feat(): add thing").is_err());
    }

    #[test]
    fn rejects_unsupported_type() {
        assert!(validate("style(ui): tweak button").is_err());
    }

    #[test]
    fn rejects_breaking_marker() {
        assert!(validate("feat(api)!: change contract").is_err());
    }

    #[test]
    fn rejects_empty_subject() {
        assert!(validate("fix(api): ").is_err());
    }

    #[test]
    fn rejects_multiline_output() {
        assert!(validate("fix(api): update request\n\nbody").is_err());
    }

    #[test]
    fn rejects_markdown_output() {
        assert!(validate("**fix(api): update request**").is_err());
    }

    #[test]
    fn rejects_explanatory_output() {
        assert!(validate("Here is the commit message: fix(api): update request").is_err());
    }
}
