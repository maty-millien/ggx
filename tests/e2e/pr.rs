use crate::support::{Env, generated, section};
use serde_json::json;

const BODY: &str = "## Summary\n\nAdds lib.\n\n## Changes\n\n- lib.rs";
const URL: &str = "https://github.com/owner/repo/pull/1";

fn feature(env: &Env) {
    env.git(&["checkout", "-b", "feature"]);
    env.write("lib.rs", "pub fn lib() {}\n");
    env.commit_all("feat(lib): add lib");
}

fn create_args(base: &str, head: &str, title: &str, body: &str) -> Vec<String> {
    [
        "pr", "create", "--base", base, "--head", head, "--title", title, "--body", body,
    ]
    .map(str::to_string)
    .to_vec()
}

#[test]
fn opens_a_pull_request_for_committed_work() {
    let env = Env::repo();
    feature(&env);
    env.respond("gh", "[]\n");
    env.reply(generated(None, None, Some(("Add lib", BODY))));
    env.respond("gh", &format!("{URL}\n"));

    let run = env.run(&["pr", "-y"]);

    let stdout = run.success();
    for expected in [
        "│ Changes\n│   A  lib.rs +1\n",
        "│ Title\n│ Add lib\n",
        "│ Body\n│ ## Summary\n│\n│ Adds lib.\n│\n│ ## Changes\n│\n│ - lib.rs\n│\n",
        "+ Pushed to origin/feature\n",
        &format!("+ Created PR {URL}\n"),
    ] {
        assert!(
            stdout.contains(expected),
            "missing {expected} in:\n{stdout}"
        );
    }
    let calls = env.calls("gh");
    assert_eq!(calls[0][..4], ["pr", "list", "--head", "feature"]);
    assert_eq!(calls[1], create_args("main", "feature", "Add lib", BODY));
    assert_eq!(env.remote_branches(), "feature\nmain\n");

    let prompt = &env.prompts()[0];
    assert_eq!(section(prompt, "## Pull Request Base"), "main");
    assert_eq!(section(prompt, "## Committed Changed Files"), "A\tlib.rs");
    assert!(section(prompt, "## Existing Commits").ends_with(" feat(lib): add lib"));
    assert!(section(prompt, "## Committed Diff\n").ends_with("+pub fn lib() {}"));
    for expected in [
        "Set branch to null.",
        "Set commit to null.",
        "Do not mention or close any issue; none are provided.",
    ] {
        assert!(
            prompt.contains(expected),
            "missing {expected} in:\n{prompt}"
        );
    }
    assert!(!prompt.contains("## Pending"));
}

#[test]
fn passes_draft_base_and_issues() {
    let env = Env::repo();
    env.git(&["remote", "set-head", "origin", "main"]);
    env.git(&["push", "origin", "main:dev"]);
    feature(&env);
    env.respond("gh", "[]\n");
    env.respond(
        "gh",
        &json!({
            "number": 12,
            "title": "Retries fail",
            "body": "y".repeat(9_000),
            "url": "https://github.com/owner/repo/issues/12",
        })
        .to_string(),
    );
    env.reply(generated(None, None, Some(("Add lib", BODY))));
    env.respond("gh", &format!("{URL}\n"));

    env.run(&["pr", "--draft", "--base", "dev", "--closes", "12", "-y"])
        .success();

    let calls = env.calls("gh");
    assert_eq!(
        calls[1],
        ["issue", "view", "12", "--json", "number,title,body,url"]
    );
    let mut create = create_args("dev", "feature", "Add lib", BODY);
    create.push("--draft".to_string());
    assert_eq!(calls[2], create);

    let prompt = &env.prompts()[0];
    assert_eq!(section(prompt, "## Pull Request Base"), "dev");
    assert!(prompt.contains(
        "Include GitHub closing references only for the issues listed under Issues To Close."
    ));
    assert!(prompt.contains(
        "## Issues To Close\n\n### Retries fail\n\nReference: 12\nNumber: 12\nURL: https://github.com/owner/repo/issues/12\n"
    ));
    assert_eq!(section(prompt, "### Retries fail"), "y".repeat(8_000));
}

#[test]
fn truncates_long_committed_diffs() {
    let env = Env::repo();
    env.git(&["checkout", "-b", "feature"]);
    env.write("big.txt", &"big line\n".repeat(5_000));
    env.commit_all("feat(big): add big file");
    env.respond("gh", "[]\n");
    env.reply(generated(None, None, Some(("Add big", BODY))));
    env.respond("gh", &format!("{URL}\n"));

    env.run(&["pr", "-y"]).success();

    let diff = section(&env.prompts()[0], "## Committed Diff\n").to_string();
    assert_eq!(diff.chars().count(), 16_000);
}

#[test]
fn creates_a_branch_when_run_on_the_default_branch_with_changes() {
    let env = Env::repo();
    env.write("fix.rs", "fix\n");
    env.reply(generated(
        Some("fix/crash"),
        Some("fix(core): handle crash"),
        Some(("Fix crash", BODY)),
    ));
    env.respond("gh", &format!("{URL}\n"));

    let run = env.run(&["pr", "-y"]);

    let stdout = run.success();
    for expected in [
        "│ Branch\n│ fix/crash\n",
        "│ Commit\n│ fix(core): handle crash\n",
        "+ Checked out fix/crash\n",
        "+ Committed to fix/crash\n",
        "+ Pushed to origin/fix/crash\n",
    ] {
        assert!(
            stdout.contains(expected),
            "missing {expected} in:\n{stdout}"
        );
    }
    assert_eq!(
        env.calls("gh"),
        [create_args("main", "fix/crash", "Fix crash", BODY)]
    );
    assert_eq!(env.branch(), "fix/crash");

    let prompt = &env.prompts()[0];
    assert!(prompt.contains("Set branch to a concise name"));
    assert_eq!(section(prompt, "## Pending Changed Files"), "A\tfix.rs");
    assert!(!prompt.contains("## Existing Commits"));
}

#[test]
fn commits_pending_work_on_a_feature_branch() {
    let env = Env::repo();
    feature(&env);
    env.git(&["push", "-u", "origin", "feature"]);
    env.write("more.rs", "more\n");
    env.respond("gh", "[]\n");
    env.reply(generated(
        None,
        Some("feat(lib): add more"),
        Some(("Add lib", BODY)),
    ));
    env.respond("gh", &format!("{URL}\n"));

    let stdout = env.run(&["pr", "-y"]).success().to_string();

    assert!(
        stdout.contains("│ Changes\n│   A  lib.rs +1\n│   A  more.rs +1\n"),
        "{stdout}"
    );
    assert!(stdout.contains("+ Committed to feature\n"));
    assert!(stdout.contains("+ Pushed to origin/feature\n"));
    assert_eq!(env.last_commit(), "feat(lib): add more");
    assert_eq!(
        env.git(&["rev-parse", "HEAD"]),
        env.git(&["rev-parse", "origin/feature"])
    );
    let prompt = &env.prompts()[0];
    assert_eq!(section(prompt, "## Committed Changed Files"), "A\tlib.rs");
    assert_eq!(section(prompt, "## Pending Changed Files"), "A\tmore.rs");
}

#[test]
fn pushes_with_the_existing_upstream() {
    let env = Env::repo();
    feature(&env);
    env.git(&["push", "-u", "origin", "feature"]);
    env.write("lib.rs", "pub fn lib() -> u8 { 1 }\n");
    env.commit_all("feat(lib): return one");
    env.respond("gh", "[]\n");
    env.reply(generated(None, None, Some(("Add lib", BODY))));
    env.respond("gh", &format!("{URL}\n"));

    let stdout = env.run(&["pr", "-y"]).success().to_string();

    assert!(stdout.contains("+ Pushed to origin/feature\n"), "{stdout}");
    assert_eq!(
        env.git(&["rev-parse", "HEAD"]),
        env.git(&["rev-parse", "origin/feature"])
    );
}

#[test]
fn refuses_when_a_pull_request_is_already_open() {
    let env = Env::repo();
    feature(&env);
    env.respond(
        "gh",
        r#"[{"number":3,"title":"Open","url":"https://github.com/owner/repo/pull/3"}]"#,
    );

    env.run(&["pr", "-y"]).failure(
        "A pull request is already open for this branch: https://github.com/owner/repo/pull/3",
    );
    assert!(env.calls("curl").is_empty());
}

#[test]
fn reports_gh_failures() {
    let env = Env::repo();
    feature(&env);
    env.respond_with("gh", "", "HTTP 401: Bad credentials\n", 1);

    env.run(&["pr", "-y"]).failure(
        "gh pr list --head feature --state open --limit 1 --json url failed: HTTP 401: Bad credentials",
    );
}

#[test]
fn rejects_invalid_workflows() {
    let env = Env::repo();
    env.run(&["pr", "-y"])
        .failure("No pending changes found on base branch 'main'.");
    env.run(&["pr", "--base", "dev", "-y"]).failure(
        "Current branch 'main' is the default base branch. Checkout 'dev' or a feature branch first.",
    );

    env.git(&["checkout", "-b", "feature"]);
    env.respond("gh", "[]\n");
    env.run(&["pr", "-y"])
        .failure("No changes found between main and feature.");
    env.respond("gh", "[]\n");
    env.run(&["pr", "--base", "nope", "-y"])
        .failure("Base branch 'nope' was not found locally or on origin.");

    env.git(&["checkout", "--detach"]);
    env.run(&["pr", "-y"])
        .failure("Cannot create a PR from detached HEAD.");
}

#[test]
fn rejects_invalid_pull_request_output() {
    let env = Env::repo();
    feature(&env);
    let cases = [
        (
            generated(None, None, None),
            "Generated output must include pull_request.",
        ),
        (
            json!({"pull_request": {"body": "b"}}),
            "Generated pull request must include a title.",
        ),
        (
            json!({"pull_request": {"title": "t"}}),
            "Generated pull request must include a body.",
        ),
        (
            generated(None, None, Some(("  ", "b"))),
            "Generated pull request title and body must not be empty.",
        ),
        (
            generated(None, None, Some(("t", "\n"))),
            "Generated pull request title and body must not be empty.",
        ),
    ];

    for (output, error) in cases {
        env.respond("gh", "[]\n");
        env.reply(output.clone());
        env.reply(output);
        env.run(&["pr", "-y"]).failure(error);
    }
}

#[test]
fn describes_the_action_before_confirming() {
    let env = Env::repo();
    env.write("fix.rs", "fix\n");
    env.reply(generated(
        Some("fix/x"),
        Some("fix(x): fix"),
        Some(("Fix", BODY)),
    ));
    let stdout = env.run_with_input(&["pr"], "n").success().to_string();
    assert!(
        stdout.contains("  ● Create fix/x, commit, push, and create PR into main\n"),
        "{stdout}"
    );
    assert!(stdout.ends_with("+ Aborted\n"));
    assert_eq!(env.branch(), "main");

    env.git(&["stash", "--include-untracked"]);
    feature(&env);
    env.respond("gh", "[]\n");
    env.reply(generated(None, None, Some(("Add lib", BODY))));
    let stdout = env.run_with_input(&["pr"], "n").success().to_string();
    assert!(
        stdout.contains("  ● Push feature and create PR into main\n"),
        "{stdout}"
    );

    env.write("more.rs", "more\n");
    env.respond("gh", "[]\n");
    env.reply(generated(
        None,
        Some("feat(lib): more"),
        Some(("Add lib", BODY)),
    ));
    let stdout = env.run_with_input(&["pr"], "n").success().to_string();
    assert!(
        stdout.contains("  ● Commit, push feature, and create PR into main\n"),
        "{stdout}"
    );

    // Only the open pull request lookups reached gh.
    assert_eq!(env.calls("gh").len(), 2);
    assert_eq!(env.remote_branches(), "main\n");
}

#[test]
fn wraps_long_lines_to_the_terminal_width() {
    let env = Env::repo();
    feature(&env);
    let words = |count: usize| ["word"; 20][..count].join(" ");
    let items = |count: usize| ["item"; 20][..count].join(" ");
    let title = format!("Add {}", words(20));
    let body = format!("{}\n{}\n  - {}", words(20), "x".repeat(100), items(20));
    env.respond("gh", "[]\n");
    env.reply(generated(None, None, Some((&title, &body))));
    env.respond("gh", &format!("{URL}\n"));

    let stdout = env.run(&["pr", "-y"]).success().to_string();

    // Without a terminal the width is 80 columns, minus 4 for the rail.
    let expected_title = format!("│ Title\n│ Add {}\n│ {}\n", words(14), words(6));
    let expected_body = format!(
        "│ Body\n│ {}\n│ {}\n│ {}\n│ {}\n│   - {}\n│   {}\n",
        words(15),
        words(5),
        "x".repeat(76),
        "x".repeat(24),
        items(14),
        items(6)
    );
    assert!(stdout.contains(&expected_title), "{stdout}");
    assert!(stdout.contains(&expected_body), "{stdout}");
}
