pub fn render(branch: &str, base: &str, commits: &str, diff: &str) -> String {
    format!(
        r#"## Instructions

Generate one concise git commit message for the squashed branch changes.
Use Conventional Commits with a non-empty scope in parentheses: feat(scope), fix(scope), refactor(scope), docs(scope), test(scope), chore(scope), build(scope), ci(scope).
Return only the commit message.
No markdown.
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

## Diff

````diff
{}
````"#,
        branch, base, commits, diff
    )
}
