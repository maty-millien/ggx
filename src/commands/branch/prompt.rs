use crate::commands::branch::context::Context;

pub fn render(context: &Context, forbidden: Option<&str>) -> String {
    let user_prompt = context.prompt.as_ref().map_or(String::new(), |prompt| {
        format!(
            r#"
## User Prompt

````
{}
````
"#,
            prompt
        )
    });

    let changes = context
        .change_source
        .map_or(String::new(), |change_source| {
            format!(
                r#"
## Changed Files ({change_source})

````
{}
````

## Diff Stat

````
{}
````

## Diff

````diff
{}
````
"#,
                context.files, context.stat, context.diff
            )
        });

    let forbidden = forbidden.map_or(String::new(), |name| {
        format!(
            r#"
## Forbidden Branch Names

````
{}
````
"#,
            name
        )
    });

    let notes = if context.diff_truncated {
        "\n\n## Notes\n\nDiff was truncated."
    } else {
        ""
    };

    format!(
        r#"## Instructions

Generate a concise git branch name.
Use exactly this format: type/short-kebab-name.
Allowed types: feat, fix, refactor, docs, test, chore.
Return only the branch name.
No markdown.
No explanation.

## Current Branch

````
{}
````{}{}{}{}"#,
        context.branch, user_prompt, changes, forbidden, notes
    )
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::commands::branch::context::Context;

    fn context() -> Context {
        Context {
            prompt: Some("name the branch".to_string()),
            branch: "main".to_string(),
            change_source: Some("staged"),
            files: "M  src/main.rs".to_string(),
            stat: "1 file changed".to_string(),
            diff: "diff --git a/src/main.rs b/src/main.rs".to_string(),
            diff_truncated: false,
        }
    }

    #[test]
    fn renders_context_prompt_changes_and_forbidden_name() {
        let output = render(&context(), Some("feat/existing"));

        assert!(output.contains("## User Prompt"));
        assert!(output.contains("name the branch"));
        assert!(output.contains("## Changed Files (staged)"));
        assert!(output.contains("M  src/main.rs"));
        assert!(output.contains("## Forbidden Branch Names"));
        assert!(output.contains("feat/existing"));
        assert!(!output.contains("Diff was truncated."));
    }

    #[test]
    fn omits_optional_sections_when_absent() {
        let output = render(
            &Context {
                prompt: None,
                change_source: None,
                diff_truncated: false,
                ..context()
            },
            None,
        );

        assert!(!output.contains("## User Prompt"));
        assert!(!output.contains("## Changed Files"));
        assert!(!output.contains("## Forbidden Branch Names"));
    }

    #[test]
    fn includes_truncation_note_when_diff_truncated() {
        let output = render(
            &Context {
                diff_truncated: true,
                ..context()
            },
            None,
        );

        assert!(output.contains("Diff was truncated."));
    }
}
