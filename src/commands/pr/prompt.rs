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

#[cfg(test)]
mod tests {
    use super::render;
    use crate::commands::pr::context::{Context, Issue};

    fn context() -> Context {
        Context {
            branch: "feature".to_string(),
            base: "main".to_string(),
            files: "M  src/main.rs".to_string(),
            stat: "1 file changed".to_string(),
            numstat: "1\t0\tsrc/main.rs".to_string(),
            commits: "abc123 feat(cli): add command".to_string(),
            diff: "diff --git a/src/main.rs b/src/main.rs".to_string(),
            issues: vec![Issue {
                reference: "#12".to_string(),
                number: "12".to_string(),
                title: "Fix thing".to_string(),
                body: "Issue body".to_string(),
                url: "https://github.com/owner/repo/issues/12".to_string(),
            }],
        }
    }

    #[test]
    fn renders_pr_context_and_issues() {
        let output = render(&context());

        assert!(output.contains("## Branch"));
        assert!(output.contains("feature"));
        assert!(output.contains("## Base"));
        assert!(output.contains("main"));
        assert!(output.contains("## Commits"));
        assert!(output.contains("abc123 feat(cli): add command"));
        assert!(output.contains("## Issues To Close"));
        assert!(output.contains("Reference: #12"));
        assert!(output.contains("Fix thing"));
    }

    #[test]
    fn omits_issues_section_when_empty() {
        let output = render(&Context {
            issues: Vec::new(),
            ..context()
        });

        assert!(!output.contains("## Issues To Close"));
    }
}
