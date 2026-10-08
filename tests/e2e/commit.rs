use crate::support::{Env, generated, section};
use serde_json::json;

#[test]
fn commits_and_pushes_all_changes() {
    let env = Env::repo();
    env.write("src/new.rs", "fn main() {}\n");
    env.write("README.md", "# Demo\nMore\n");
    env.reply(generated(None, Some("feat(cli): add entry point"), None));

    let run = env.run(&["commit", "-y"]);

    let stdout = run.success();
    assert!(
        stdout.contains("│ Changes\n│   M  README.md +1\n│   A  src/new.rs +1\n│\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("│ feat(cli): add entry point\n"),
        "{stdout}"
    );
    assert!(
        stdout.ends_with("+ Committed to main\n│\n+ Pushed to origin/main\n"),
        "{stdout}"
    );
    assert_eq!(env.last_commit(), "feat(cli): add entry point");
    assert_eq!(env.git(&["status", "--porcelain"]), "");
    assert_eq!(
        env.git(&["rev-parse", "HEAD"]),
        env.git(&["rev-parse", "origin/main"])
    );
}

#[test]
fn sends_the_pending_changes_to_the_model() {
    let env = Env::repo();
    env.write("docs/notes.md", "hello\n");
    env.reply(generated(None, Some("docs(notes): add notes"), None));

    env.run(&["commit", "-y"]).success();

    let prompt = &env.prompts()[0];
    assert!(prompt.starts_with("Do not invoke tools.\n"), "{prompt}");
    for expected in [
        "Set branch to null.",
        "Set commit to one Conventional Commit line using type(scope): subject.",
        "Set pull_request to null.",
    ] {
        assert!(
            prompt.contains(expected),
            "missing {expected} in:\n{prompt}"
        );
    }
    assert_eq!(section(prompt, "## Current Branch"), "main");
    assert_eq!(section(prompt, "## Pull Request Base"), "Not applicable");
    assert_eq!(
        section(prompt, "## Pending Changed Files"),
        "A\tdocs/notes.md"
    );
    assert_eq!(section(prompt, "## Pending Numstat"), "1\t0\tdocs/notes.md");
    assert_eq!(section(prompt, "## README"), "# Demo\n");
    assert!(section(prompt, "## Pending Diff\n").ends_with("+hello"));
    for absent in [
        "## User Prompt",
        "## Notes",
        "## Existing Commits",
        "## Issues To Close",
    ] {
        assert!(
            !prompt.contains(absent),
            "unexpected {absent} in:\n{prompt}"
        );
    }
}

#[test]
fn reads_the_readme_from_the_root_before_docs() {
    let env = Env::repo();
    env.write("docs/.keep", "");
    env.git(&["mv", "README.md", "docs/Readme.md"]);
    env.commit_all("move readme");
    env.write("change.txt", "change\n");
    env.reply(generated(None, Some("chore(repo): add change"), None));
    env.run(&["commit", "-y"]).success();

    env.write("readme.TXT", "# Root\n");
    env.reply(generated(None, Some("docs(repo): add root readme"), None));
    env.run(&["commit", "-y"]).success();

    let prompts = env.prompts();
    assert_eq!(section(&prompts[0], "## README"), "# Demo\n");
    assert_eq!(section(&prompts[1], "## README"), "# Root\n");
}

#[test]
fn notes_truncated_context() {
    let env = Env::repo();
    env.write("README.md", &"r".repeat(9_000));
    env.write(
        "big.txt",
        &"big line, see diff --git output\n".repeat(2_000),
    );
    env.write("small.txt", "small change\n");
    env.reply(generated(None, Some("chore(repo): add files"), None));

    env.run(&["commit", "-y"]).success();

    let prompt = &env.prompts()[0];
    assert_eq!(section(prompt, "## README"), "r".repeat(8_000));
    assert!(prompt.contains(
        "## Notes\n\nDiff exceeded context budget.\nOne or more file diffs were truncated.\nREADME was truncated.\n"
    ));
    let diff = section(prompt, "## Pending Diff\n");
    assert!(diff.chars().count() <= 16_000);
    assert!(diff.contains("diff --git a/README.md b/README.md"));
    assert!(diff.contains("diff --git a/big.txt b/big.txt"));
    assert!(diff.contains("+small change"));
}

#[test]
fn asks_before_committing() {
    let env = Env::repo();
    env.git(&["checkout", "-b", "feature"]);
    env.write("staged.txt", "staged\n");
    env.git(&["add", "staged.txt"]);
    env.write("unstaged.txt", "unstaged\n");
    let staged = env.git(&["diff", "--staged", "--name-status"]);
    env.reply(generated(None, Some("feat(files): add files"), None));

    let run = env.run_with_input(&["commit"], "2\n");

    let stdout = run.success();
    assert!(
        stdout.contains(
            "+ What would you like to do?\n  ● Commit and push to origin/feature\n  ○ Cancel\n"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("  ○ Commit and push to origin/feature\n  ● Cancel\n"));
    assert!(stdout.ends_with("+ What would you like to do?\n│ Cancel\n│\n+ Aborted\n"));
    assert_eq!(env.last_commit(), "initial");
    assert_eq!(env.git(&["diff", "--staged", "--name-status"]), staged);
    let files = section(&env.prompts()[0], "## Pending Changed Files").to_string();
    assert_eq!(files, "A\tstaged.txt\nA\tunstaged.txt");
}

#[test]
fn commits_without_pushing_when_there_is_no_origin() {
    let env = Env::repo();
    env.git(&["remote", "remove", "origin"]);
    env.write("a.txt", "a\n");
    env.reply(generated(None, Some("feat(a): add a"), None));

    let run = env.run_with_input(&["commit"], "y");

    let stdout = run.success();
    assert!(stdout.contains("  ● Commit to main\n"), "{stdout}");
    assert!(stdout.contains("+ Committed to main\n"));
    assert!(!stdout.contains("Pushed"));
    assert_eq!(env.last_commit(), "feat(a): add a");
}

#[test]
fn retries_once_with_the_rejection_reason() {
    let env = Env::repo();
    env.write("a.txt", "a\n");
    env.reply(generated(None, Some("update stuff"), None));
    env.reply(generated(None, Some("feat(a): add a"), None));

    env.run(&["commit", "-y"]).success();

    let prompts = env.prompts();
    assert!(!prompts[0].contains("## Previous Attempt"));
    assert!(prompts[1].ends_with(
        "\n## Previous Attempt\n\nThe previous response was rejected: Commit message must use 'type(scope): subject'.\nReturn a corrected, fully regenerated object.\n"
    ));
    assert_eq!(env.last_commit(), "feat(a): add a");
}

#[test]
fn rejects_invalid_commit_messages() {
    let env = Env::repo();
    env.write("a.txt", "a\n");
    let cases = [
        (
            generated(None, Some(""), None),
            "Commit message must be exactly one line.",
        ),
        (
            generated(None, Some("feat(a): one\ntwo"), None),
            "Commit message must be exactly one line.",
        ),
        (
            generated(None, Some("feat(a): one\rtwo"), None),
            "Commit message must be exactly one line.",
        ),
        (
            generated(None, Some("update stuff"), None),
            "Commit message must use 'type(scope): subject'.",
        ),
        (
            generated(None, Some("feat: add"), None),
            "Commit message must include a non-empty scope.",
        ),
        (
            generated(None, Some("wip(a): add"), None),
            "Commit message type 'wip' is not allowed.",
        ),
        (
            generated(None, Some("feat(a: add"), None),
            "Commit message scope must close before the colon.",
        ),
        (
            generated(None, Some("feat( ): add"), None),
            "Commit message scope cannot be empty.",
        ),
        (
            generated(None, Some("feat((a)): add"), None),
            "Commit message scope cannot be empty.",
        ),
        (
            json!({"branch": null}),
            "Generated output must include commit.",
        ),
        (
            json!({"commit": 1}),
            "Generated output must include commit.",
        ),
    ];

    for (output, error) in cases {
        env.reply(output.clone());
        env.reply(output);
        env.run(&["commit", "-y"]).failure(error);
    }

    env.reply_text("not json");
    env.reply_text("not json");
    env.run(&["commit", "-y"])
        .failure("expected ident at line 1 column 2");
    assert_eq!(env.last_commit(), "initial");
}

#[test]
fn accepts_fenced_json_and_every_allowed_type() {
    let env = Env::repo();

    for (index, kind) in [
        "feat", "fix", "refactor", "docs", "test", "chore", "build", "ci",
    ]
    .into_iter()
    .enumerate()
    {
        env.write(&format!("{kind}.txt"), "x\n");
        let message = format!("{kind}(scope): change {index}");
        let output = generated(None, Some(&message), None);
        let fence = if index % 2 == 0 { "```json" } else { "```" };
        env.reply_text(&format!("{fence}\n{output}\n```"));

        env.run(&["commit", "-y"]).success();
        assert_eq!(env.last_commit(), message);
    }
}

#[test]
fn refuses_to_commit_with_conflicts() {
    let env = Env::repo();
    env.git(&["checkout", "-b", "other"]);
    env.write("README.md", "other\n");
    env.commit_all("other");
    env.git(&["checkout", "main"]);
    env.write("README.md", "main\n");
    env.commit_all("main");
    assert!(!env.git_status(&["merge", "other"]));

    env.run(&["commit", "-y"])
        .failure("Resolve conflicts before committing.");
}

#[test]
fn requires_changes() {
    Env::repo()
        .run(&["commit", "-y"])
        .failure("No changes found.");
}

#[test]
fn shows_deleted_renamed_and_retyped_files() {
    let env = Env::repo();
    env.write("deleted.txt", "gone\n");
    env.write("old.txt", "same\n");
    env.write("retyped.txt", "file\n");
    env.commit_all("files");
    env.git(&["rm", "--quiet", "deleted.txt"]);
    env.git(&["mv", "old.txt", "new.txt"]);
    std::fs::remove_file(env.repo.join("retyped.txt")).unwrap();
    std::os::unix::fs::symlink("README.md", env.repo.join("retyped.txt")).unwrap();
    env.reply(generated(None, Some("chore(files): reshuffle"), None));

    let stdout = env.run(&["commit", "-y"]).success().to_string();

    for expected in [
        "│   D  deleted.txt -1\n",
        "│   R  new.txt\n",
        "│   ?  retyped.txt",
    ] {
        assert!(
            stdout.contains(expected),
            "missing {expected} in:\n{stdout}"
        );
    }
}
