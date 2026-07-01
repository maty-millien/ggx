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
