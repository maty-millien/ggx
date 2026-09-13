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

    fn error(message: &str) -> String {
        validate(message).unwrap_err().to_string()
    }

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
    fn rejects_empty_or_multiline_messages() {
        assert_eq!(error(""), "Commit message must be exactly one line.");
        assert_eq!(
            error("fix(api): update request\n\nbody"),
            "Commit message must be exactly one line."
        );
        assert_eq!(
            error("fix(api): update\r"),
            "Commit message must be exactly one line."
        );
    }

    #[test]
    fn rejects_missing_type_separator() {
        assert_eq!(
            error("update request"),
            "Commit message must use 'type(scope): subject'."
        );
        assert_eq!(
            error("fix(api):no space"),
            "Commit message must use 'type(scope): subject'."
        );
    }

    #[test]
    fn rejects_empty_subject() {
        assert_eq!(
            error("fix(api): "),
            "Commit message subject cannot be empty."
        );
    }

    #[test]
    fn rejects_missing_scope() {
        assert_eq!(
            error("feat: add thing"),
            "Commit message must include a non-empty scope."
        );
    }

    #[test]
    fn rejects_unsupported_type() {
        assert_eq!(
            error("style(ui): tweak button"),
            "Commit message type 'style' is not allowed."
        );
    }

    #[test]
    fn rejects_unclosed_scope() {
        assert_eq!(
            error("feat(api)!: change contract"),
            "Commit message scope must close before the colon."
        );
    }

    #[test]
    fn rejects_empty_or_nested_scope() {
        assert_eq!(
            error("feat(): add thing"),
            "Commit message scope cannot be empty."
        );
        assert_eq!(
            error("feat( ): add thing"),
            "Commit message scope cannot be empty."
        );
        assert_eq!(
            error("feat(a(b)): add thing"),
            "Commit message scope cannot be empty."
        );
    }
}
