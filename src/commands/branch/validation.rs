const PREFIXES: &[&str] = &["feat", "fix", "refactor", "docs", "test", "chore"];

pub fn normalize(raw: &str) -> anyhow::Result<String> {
    let candidate = first_content_line(raw);
    let mut normalized = String::new();
    let mut previous_dash = false;

    for character in candidate.chars() {
        let character = character.to_ascii_lowercase();

        if character.is_ascii_alphanumeric() || character == '/' {
            normalized.push(character);
            previous_dash = false;
        } else if character == '-' && !previous_dash {
            normalized.push(character);
            previous_dash = true;
        }
    }

    let Some((prefix, slug)) = normalized.split_once('/') else {
        anyhow::bail!("Generated branch name must use type/slug format.");
    };

    if !PREFIXES.contains(&prefix) {
        anyhow::bail!("Generated branch name used unsupported type '{}'.", prefix);
    }

    if slug.contains('/') {
        anyhow::bail!("Generated branch name must contain only one slash.");
    }

    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        anyhow::bail!("Generated branch name must include a slug.");
    }

    Ok(format!("{}/{}", prefix, slug))
}

fn first_content_line(raw: &str) -> &str {
    raw.trim()
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("```"))
        .unwrap_or_default()
}
