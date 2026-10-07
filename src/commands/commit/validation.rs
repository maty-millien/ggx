const ALLOWED_TYPES: &[&str] = &[
    "feat", "fix", "refactor", "docs", "test", "chore", "build", "ci",
];

pub fn validate(message: &str) -> anyhow::Result<()> {
    if message.is_empty() || message.contains('\n') || message.contains('\r') {
        anyhow::bail!("Commit message must be exactly one line.");
    }

    // The message is trimmed, so a subject after ": " is never empty.
    let Some((kind, _)) = message.split_once(": ") else {
        anyhow::bail!("Commit message must use 'type(scope): subject'.");
    };

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
