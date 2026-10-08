use crate::{Env, generated, section};
use serde_json::json;

#[test]
fn creates_commits_and_pushes_a_branch_for_pending_changes() {
    let env = Env::repo();
    env.write("retry.rs", "// retry\n");
    env.reply(generated(
        Some("Feat/Add--Retries"),
        Some("feat(api): add retries"),
        None,
    ));

    let run = env.run(&["branch", "-y"]);

    let stdout = run.success();
    for expected in [
        "│ Changes\n│   A  retry.rs +1\n",
        "│ Branch\n│ feat/add-retries\n",
        "│ Commit\n│ feat(api): add retries\n",
        "+ Checked out feat/add-retries\n",
        "+ Committed to feat/add-retries\n",
        "+ Pushed to origin/feat/add-retries\n",
    ] {
        assert!(
            stdout.contains(expected),
            "missing {expected} in:\n{stdout}"
        );
    }
    assert_eq!(env.branch(), "feat/add-retries");
    assert_eq!(env.last_commit(), "feat(api): add retries");
    assert_eq!(env.remote_branches(), "feat/add-retries\nmain\n");

    let prompt = &env.prompts()[0];
    assert!(prompt.contains("Set branch to a concise name using type/short-kebab-name."));
    assert!(prompt.contains("Set commit to one Conventional Commit line"));
    assert_eq!(section(prompt, "## Pending Changed Files"), "A\tretry.rs");
}

#[test]
fn creates_a_branch_from_a_prompt_alone() {
    let env = Env::repo();
    env.reply(generated(Some("docs/usage-guide"), None, None));

    let run = env.run(&["branch", "  write the usage guide  ", "-y"]);

    let stdout = run.success();
    assert!(
        stdout.contains("+ Checked out docs/usage-guide\n"),
        "{stdout}"
    );
    assert!(stdout.contains("+ Pushed to origin/docs/usage-guide\n"));
    assert!(!stdout.contains("│ Commit\n"));
    assert_eq!(env.branch(), "docs/usage-guide");
    assert_eq!(env.last_commit(), "initial");
    assert_eq!(env.remote_branches(), "docs/usage-guide\nmain\n");

    let prompt = &env.prompts()[0];
    assert_eq!(section(prompt, "## User Prompt"), "write the usage guide");
    assert!(prompt.contains("Set commit to null."));
    assert!(!prompt.contains("## Pending"));
}

#[test]
fn requires_a_prompt_or_changes() {
    Env::repo()
        .run(&["branch", "   ", "-y"])
        .failure("No staged or unstaged changes found.");
}

#[test]
fn retries_when_the_branch_exists_locally_or_on_origin() {
    let env = Env::repo();
    env.git(&["branch", "feat/local"]);
    env.git(&["push", "origin", "main:feat/remote"]);
    env.reply(generated(Some("feat/local"), None, None));
    env.reply(generated(Some("feat/remote"), None, None));

    env.run(&["branch", "add things", "-y"])
        .failure("Branch 'feat/remote' already exists.");
    assert!(
        env.prompts()[1]
            .contains("The previous response was rejected: Branch 'feat/local' already exists.")
    );

    env.reply(generated(Some("feat/remote"), None, None));
    env.reply(generated(Some("feat/fresh"), None, None));
    env.run(&["branch", "add things", "-y"]).success();
    assert_eq!(env.branch(), "feat/fresh");
}

#[test]
fn rejects_invalid_branch_names() {
    let env = Env::repo();
    let cases = [
        (
            generated(Some("wip/thing"), None, None),
            "Generated branch name used unsupported type 'wip'.",
        ),
        (
            generated(Some("thing"), None, None),
            "Generated branch name must use type/slug format.",
        ),
        (
            generated(Some(""), None, None),
            "Generated branch name must use type/slug format.",
        ),
        (
            generated(Some("feat/a/b"), None, None),
            "Generated branch name must contain only one slash.",
        ),
        (
            generated(Some("feat/---"), None, None),
            "Generated branch name must include a slug.",
        ),
        (
            json!({"commit": null}),
            "Generated output must include branch.",
        ),
    ];

    for (output, error) in cases {
        env.reply(output.clone());
        env.reply(output);
        env.run(&["branch", "x", "-y"]).failure(error);
    }
    assert_eq!(env.branches(), "main\n");
}

#[test]
fn normalizes_fenced_branch_names() {
    let env = Env::repo();
    env.reply(generated(Some("```\n\n  Fix/Login Bug!\n```"), None, None));

    env.run(&["branch", "x", "-y"]).success();

    assert_eq!(env.branch(), "fix/loginbug");
}

#[test]
fn asks_before_creating_the_branch() {
    let env = Env::repo();
    env.reply(generated(Some("docs/guide"), None, None));
    let stdout = env
        .run_with_input(&["branch", "guide"], "n")
        .success()
        .to_string();
    assert!(
        stdout.contains("  ● Create, checkout, and push docs/guide\n"),
        "{stdout}"
    );
    assert!(stdout.ends_with("+ Aborted\n"));

    env.write("a.txt", "a\n");
    env.reply(generated(Some("feat/a"), Some("feat(a): add a"), None));
    let stdout = env.run_with_input(&["branch"], "q").success().to_string();
    assert!(
        stdout.contains("  ● Create, checkout, commit, and push feat/a\n"),
        "{stdout}"
    );
    assert!(stdout.ends_with("+ Aborted\n"));

    assert_eq!(env.branches(), "main\n");
    assert_eq!(env.last_commit(), "initial");
}

#[test]
fn commits_without_pushing_when_there_is_no_origin() {
    let env = Env::repo();
    env.git(&["remote", "remove", "origin"]);
    env.write("a.txt", "a\n");
    env.reply(generated(Some("feat/a"), Some("feat(a): add a"), None));

    let stdout = env.run(&["branch", "-y"]).success().to_string();

    assert!(stdout.ends_with("+ Committed to feat/a\n"), "{stdout}");
    assert_eq!(env.branch(), "feat/a");
    assert_eq!(env.last_commit(), "feat(a): add a");
}
