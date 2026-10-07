use crate::support::Env;

/// Adds a branch with one commit, pushed to origin, then deleted on origin.
fn gone_branch(env: &Env, name: &str) {
    env.git(&["checkout", "-b", name, "main"]);
    env.write(&format!("{name}.txt"), "x\n");
    env.commit_all(name);
    env.git(&["push", "-u", "origin", name]);
    env.git_in(&env.root.join("remote.git"), &["branch", "-D", name]);
    env.git(&["checkout", "main"]);
}

#[test]
fn cleans_merged_and_gone_branches_then_restores_the_starting_branch() {
    let env = Env::repo();
    env.git(&["branch", "merged"]);
    gone_branch(&env, "gone");
    env.git(&["checkout", "-b", "work"]);
    env.write("work.txt", "x\n");
    env.commit_all("work");

    let run = env.run(&["sync", "-y"]);

    let stdout = run.success();
    for expected in [
        "+ Sync complete",
        "│ Branches\n│ gone\n│ merged\n│\n",
        "+ Deleted gone\n",
        "+ Deleted merged\n",
    ] {
        assert!(
            stdout.contains(expected),
            "missing {expected} in:\n{stdout}"
        );
    }
    assert!(stdout.ends_with("+ Checked out work\n"));
    assert_eq!(env.branches(), "main\nwork\n");
    assert_eq!(env.branch(), "work");
}

#[test]
fn deletes_the_starting_branch_when_its_remote_is_gone() {
    let env = Env::repo();
    gone_branch(&env, "gone");
    env.git(&["checkout", "gone"]);

    let stdout = env.run(&["sync", "-y"]).success().to_string();

    assert!(stdout.ends_with("+ Deleted gone\n"), "{stdout}");
    assert_eq!(env.branches(), "main\n");
    assert_eq!(env.branch(), "main");
}

#[test]
fn keeps_the_starting_branch_when_merged() {
    let env = Env::repo();
    env.git(&["checkout", "-b", "merged"]);

    let stdout = env.run(&["sync", "-y"]).success().to_string();

    assert!(
        stdout.contains("+ No local branches to clean\n"),
        "{stdout}"
    );
    assert!(stdout.ends_with("+ Checked out merged\n"));
    assert_eq!(env.branch(), "merged");
}

#[test]
fn pulls_the_default_branch() {
    let env = Env::repo();
    let other = env.root.join("other");
    env.git_in(&env.root, &["clone", "remote.git", "other"]);
    env.write_file(&other.join("new.txt"), "new\n");
    env.git_in(&other, &["add", "--all"]);
    env.git_in(&other, &["commit", "-m", "remote change"]);
    env.git_in(&other, &["push"]);

    let stdout = env.run(&["sync", "-y"]).success().to_string();

    assert!(
        stdout.ends_with("+ No local branches to clean\n"),
        "{stdout}"
    );
    assert_eq!(env.last_commit(), "remote change");
}

#[test]
fn asks_before_deleting_branches() {
    let env = Env::repo();
    env.git(&["branch", "one"]);
    let stdout = env.run_with_input(&["sync"], "n").success().to_string();
    assert!(stdout.contains("  ● Delete 1 local branch\n"), "{stdout}");
    assert!(stdout.ends_with("+ Cleanup skipped\n"));

    env.git(&["branch", "two"]);
    let stdout = env.run_with_input(&["sync"], "n").success().to_string();
    assert!(stdout.contains("  ● Delete 2 local branches\n"), "{stdout}");
    assert_eq!(env.branches(), "main\none\ntwo\n");
}

#[test]
fn requires_a_clean_worktree() {
    let env = Env::repo();
    env.write("README.md", "changed\n");

    env.run(&["sync", "-y"])
        .failure("Working tree is not clean. Commit or stash your changes first.");
}
