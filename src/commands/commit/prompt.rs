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
        notes.push("Diff was truncated.");
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
        r#"## Instructions

Generate a concise git commit message for the {change_source} changes.
Use Conventional Commits with a non-empty scope in parentheses: feat(scope), fix(scope), refactor(scope), docs(scope), test(scope), chore(scope), build(scope), ci(scope).
Return only the commit message.
No markdown.
No explanation.

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
{}

## Diff

````diff
{}
````{}"#,
        context.branch,
        context.files,
        context.stat,
        readme,
        context.diff,
        notes,
        change_source = context.change_source
    )
}
