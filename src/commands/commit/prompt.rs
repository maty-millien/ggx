use crate::commands::commit::context::Context;

pub fn render(context: &Context) -> String {
    let readme = context.readme.as_ref().map_or(String::new(), |readme| {
        format!(
            r#"
## README

````markdown
{}
````
"#,
            readme
        )
    });

    let mut notes = Vec::new();
    if context.diff_truncated {
        notes.push("Diff exceeded context budget.");
    }
    if context.diff_file_truncated {
        notes.push("One or more file diffs were truncated.");
    }
    if context.readme_truncated {
        notes.push("README was truncated.");
    }
    let notes = if notes.is_empty() {
        String::new()
    } else {
        format!("\n\n## Notes\n\n{}", notes.join("\n"))
    };

    format!(
        r#"## Output Contract

Return exactly one line matching this structure:
<type>(<scope>): <subject>

Rules:
- `<type>` must be one of: feat, fix, refactor, docs, test, chore, build, ci
- `<scope>` must be present and non-empty
- Use exactly `): ` between the scope and subject
- `<subject>` must be concise and non-empty
- Do not use markdown, quotes, prefixes, explanations, or additional lines

Valid: `feat(cli): add interactive commit confirmation`
Valid: `docs(readme): clarify installation steps`
Invalid: `feat: add confirmation` because the scope is missing
Invalid: `feat(cli):add confirmation` because the space after the colon is missing
Invalid: `Here is the message: feat(cli): add confirmation` because it contains a prefix

## Branch

````
{}
````

## Changed Files

````
{}
````

## Diff Stat

````
{}
````

## Numstat

````
{}
````

## Diff Summary

````
{}
````
{}

## Diff

````diff
{}
````{}

## Required Output

Based on the repository context above, output only the commit message now.
Remember: exactly one line in the form `<type>(<scope>): <subject>`."#,
        context.branch,
        context.files,
        context.stat,
        context.numstat,
        context.summary,
        readme,
        context.diff,
        notes
    )
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::commands::commit::context::Context;

    fn context() -> Context {
        Context {
            branch: "feature".to_string(),
            files: "M  src/main.rs".to_string(),
            stat: "1 file changed".to_string(),
            numstat: "1\t0\tsrc/main.rs".to_string(),
            summary: "create mode 100644 src/main.rs".to_string(),
            readme: Some("# ggx".to_string()),
            diff: "diff --git a/src/main.rs b/src/main.rs".to_string(),
            diff_truncated: false,
            diff_file_truncated: false,
            readme_truncated: false,
        }
    }

    #[test]
    fn renders_commit_context_and_readme() {
        let output = render(&context());

        assert!(output.contains("## Branch"));
        assert!(output.contains("feature"));
        assert!(output.contains("## Changed Files"));
        assert!(output.contains("M  src/main.rs"));
        assert!(output.contains("## README"));
        assert!(output.contains("# ggx"));
        assert!(output.contains("## Required Output"));
        assert!(
            output
                .ends_with("Remember: exactly one line in the form `<type>(<scope>): <subject>`.")
        );
        assert!(!output.contains("## Notes"));
    }

    #[test]
    fn omits_readme_when_absent() {
        let output = render(&Context {
            readme: None,
            ..context()
        });

        assert!(!output.contains("## README"));
    }

    #[test]
    fn renders_only_relevant_truncation_notes() {
        let output = render(&Context {
            diff_truncated: true,
            diff_file_truncated: true,
            readme_truncated: true,
            ..context()
        });

        assert!(output.contains("Diff exceeded context budget."));
        assert!(output.contains("One or more file diffs were truncated."));
        assert!(output.contains("README was truncated."));
    }
}
