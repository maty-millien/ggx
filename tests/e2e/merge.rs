use crate::support::Env;
use serde_json::json;

fn pull_request(number: u32, title: &str, merge_state: &str, review: &str) -> String {
    json!({
        "number": number,
        "title": title,
        "url": format!("https://github.com/owner/repo/pull/{number}"),
        "headRefName": "feature",
        "baseRefName": "main",
        "mergeStateStatus": merge_state,
        "reviewDecision": review,
    })
    .to_string()
}

#[test]
fn merges_and_syncs_the_base_branch() {
    let env = Env::repo();
    env.git(&["checkout", "-b", "feature"]);
    env.respond("gh", &pull_request(7, "Add merge", "CLEAN", "APPROVED"));
    env.respond("gh", "");

    let run = env.run(&["merge", "-y"]);

    let stdout = run.success();
    assert!(
        stdout.contains(
            "│ Pull Request\n│ #7  Add merge\n│ https://github.com/owner/repo/pull/7\n│\n│ Branch      feature → main\n│ Merge state Clean\n│ Review      Approved\n│\n"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("+ Merged PR #7\n"));
    assert!(stdout.ends_with("+ Synced main\n"));
    let calls = env.calls("gh");
    assert_eq!(calls[0][..2], ["pr", "view"]);
    assert_eq!(calls[1], ["pr", "merge", "--merge", "--delete-branch"]);
    assert_eq!(env.branch(), "main");
}

#[test]
fn squash_merges_and_keeps_the_branch_with_admin() {
    let env = Env::repo();
    env.respond("gh", &pull_request(8, "Add squash", "BLOCKED", ""));
    env.respond("gh", "");

    let run = env.run(&["squash", "--keep-branch", "--admin", "-y"]);

    let stdout = run.success();
    assert!(stdout.contains("│ Merge state Blocked\n│\n"), "{stdout}");
    assert!(!stdout.contains("Review"));
    assert!(stdout.contains("+ Squash merged PR #8\n"));
    assert_eq!(env.calls("gh")[1], ["pr", "merge", "--squash", "--admin"]);
}

#[test]
fn asks_before_merging() {
    let env = Env::repo();
    env.respond("gh", &pull_request(9, "Wip", "", "CHANGES_REQUESTED"));
    env.respond("gh", &pull_request(9, "Wip", "", "CHANGES_REQUESTED"));

    let stdout = env.run_with_input(&["merge"], "n").success().to_string();
    assert!(
        stdout.contains("│ Merge state Unknown\n│ Review      Changes requested\n"),
        "{stdout}"
    );
    assert!(stdout.contains("  ● Merge PR #9 into main and delete branch\n"));
    assert!(stdout.ends_with("+ Aborted\n"));

    let stdout = env
        .run_with_input(&["squash", "--keep-branch", "--admin"], "n")
        .success()
        .to_string();
    assert!(
        stdout.contains("  ● Squash merge PR #9 into main and keep branch with admin\n"),
        "{stdout}"
    );
    assert_eq!(env.calls("gh").len(), 2);
}

#[test]
fn requires_a_clean_worktree() {
    let env = Env::repo();
    env.write("untracked.txt", "x\n");

    env.run(&["merge", "-y"])
        .failure("Working tree is not clean. Commit or stash your changes first.");
    assert!(env.calls("gh").is_empty());
}

#[test]
fn reports_merge_failures() {
    let env = Env::repo();
    env.respond("gh", &pull_request(7, "Add merge", "DIRTY", ""));
    env.respond_with("gh", "", "Pull request is not mergeable\n", 1);

    env.run(&["merge", "-y"])
        .failure("gh pr merge --merge --delete-branch failed: Pull request is not mergeable");
}
