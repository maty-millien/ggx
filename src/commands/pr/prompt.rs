use crate::commands::pr::context::{Context, Issue};

pub fn render(context: &Context) -> String {
    let issues = if context.issues.is_empty() {
        String::new()
    } else {
        format!("\n\n## Issues To Close\n{}", render_issues(&context.issues))
    };

    format!(
        r#"## Instructions

Generate a GitHub pull request title and body.
Return the title on the first line, then a blank line, then the body.
The body must be GitHub-flavored Markdown.
The body must include summary and changes sections using markdown headings.
Do not include test plan, risk, or notes sections.
If issues to close are provided, include the correct GitHub closing references in the body yourself.
Do not use markdown fences around the full response.
No explanation.

## Branch

````
{}
````

## Base

````
{}
````

## Commits

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

## Diff

````diff
{}
````{}"#,
        context.branch,
        context.base,
        context.commits,
        context.files,
        context.stat,
        context.diff,
        issues
    )
}

fn render_issues(issues: &[Issue]) -> String {
    issues
        .iter()
        .map(|issue| {
            format!(
                r#"
### {}

Reference: {}
Number: {}
URL: {}

````
{}
````
"#,
                issue.title, issue.reference, issue.number, issue.url, issue.body
            )
        })
        .collect::<Vec<_>>()
        .join("")
}
