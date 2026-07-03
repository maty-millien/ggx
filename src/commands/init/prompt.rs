use crate::commands::init::config::Settings;

pub fn render(
    settings: &Settings,
    project_name: &str,
    readme: Option<&str>,
    recent_commits: &str,
) -> String {
    let readme_section = readme.map_or(String::new(), |readme| {
        format!(
            r#"

## README

````markdown
{}
````"#,
            readme
        )
    });

    let commits_section = if recent_commits.trim().is_empty() {
        String::new()
    } else {
        format!(
            r#"

## Recent commits

````
{}
````"#,
            recent_commits
        )
    };

    format!(
        r#"## Instructions

Write a CONTRIBUTING.md for the project "{project_name}".
Explain how to contribute and document the conventions below in clear prose.
Use Markdown with section headings. Be concise and practical.
Return only the Markdown document. No explanation. Do not wrap the whole document in a code fence.

## Conventions

- Commit convention: {convention}
- Squash pull requests when merging: {squash}
- Default base branch: {base}
- Open pull requests as drafts by default: {draft}
- Push policy: {push}{readme_section}{commits_section}"#,
        project_name = project_name,
        convention = settings.commit_convention,
        squash = yes_no(settings.squash_on_merge),
        base = settings.base_branch,
        draft = yes_no(settings.open_as_draft),
        push = settings.push_policy,
        readme_section = readme_section,
        commits_section = commits_section,
    )
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::commands::init::config::Settings;

    fn settings() -> Settings {
        Settings {
            commit_convention: "Conventional Commits".to_string(),
            squash_on_merge: true,
            base_branch: "develop".to_string(),
            open_as_draft: false,
            push_policy: "PR-only".to_string(),
        }
    }

    #[test]
    fn renders_conventions_and_project_name() {
        let output = render(&settings(), "ggx", Some("# ggx"), "abc123 feat: thing");

        assert!(output.contains(r#"project "ggx""#));
        assert!(output.contains("Commit convention: Conventional Commits"));
        assert!(output.contains("Squash pull requests when merging: yes"));
        assert!(output.contains("Default base branch: develop"));
        assert!(output.contains("Open pull requests as drafts by default: no"));
        assert!(output.contains("Push policy: PR-only"));
        assert!(output.contains("## README"));
        assert!(output.contains("# ggx"));
        assert!(output.contains("## Recent commits"));
    }

    #[test]
    fn omits_optional_sections_when_absent() {
        let output = render(&settings(), "ggx", None, "");

        assert!(!output.contains("## README"));
        assert!(!output.contains("## Recent commits"));
    }
}
