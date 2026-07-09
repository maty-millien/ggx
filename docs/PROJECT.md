# ggx Project

ggx is a fast Rust git workflow CLI with AI generated branches, commits, and PR copy, plus GitHub PR merge and squash flows.

## Commands

| Command      | Purpose                                                                     |
| ------------ | --------------------------------------------------------------------------- |
| `ggx branch` | Generate a branch, commit pending changes, and push                         |
| `ggx commit` | Preview all changes, confirm, commit, auto push if origin exists            |
| `ggx pr`     | Push the current branch and create a GitHub pull request                    |
| `ggx sync`   | Sync the default branch and clean safe local branches                       |
| `ggx merge`  | Merge branch or PR, delete branch by default, checkout default branch, sync |
| `ggx squash` | Squash merge the current GitHub pull request                                |
| `ggx -v`     | Print the ggx version                                                       |

## Key Defaults

| Behavior                           | Default                          |
| ---------------------------------- | -------------------------------- |
| Commit input                       | All changes                      |
| Commit push                        | Auto push if origin exists       |
| Missing remote                     | Skip push                        |
| Branch deletion after merge        | Enabled                          |
| Remote branch deletion after merge | Enabled                          |
| Merge behavior                     | Full land flow                   |
| Merge strategy                     | Normal merge only                |
| Squash flow                        | Separate `ggx squash` command    |
| Destructive actions                | Always ask with an action prompt |
| Terminal input                     | Suppressed except action prompts |
| Sync cleanup                       | Confirm before deleting          |
| Protected branch merges            | Use `--admin` when needed        |

## Command Flags

| Flag              | Purpose                                                                |
| ----------------- | ---------------------------------------------------------------------- |
| `--draft`         | Create a draft PR with `ggx pr`                                        |
| `--closes`        | Include issue context in a generated PR body with `ggx pr`             |
| `--keep-branch`   | Do not delete the branch after `ggx merge` or `ggx squash`             |
| `--admin`         | Pass admin privileges to `gh pr merge` for `ggx merge` or `ggx squash` |
| `--version`, `-v` | Print the ggx version                                                  |

## Common Workflows

| Workflow                               | Command                                    |
| -------------------------------------- | ------------------------------------------ |
| Create branch from current changes     | `ggx branch`                               |
| Create branch from prompt              | `ggx branch "add stripe webhook handling"` |
| Preview, stage, and commit all changes | `ggx commit`                               |
| Create PR                              | `ggx pr`                                   |
| Create draft PR                        | `ggx pr --draft`                           |
| Include issue context in PR text       | `ggx pr --closes 123`                      |
| Sync base branch and clean locals      | `ggx sync`                                 |
| Merge PR and clean branch              | `ggx merge`                                |
| Merge but keep branch                  | `ggx merge --keep-branch`                  |
| Squash merge current PR                | `ggx squash`                               |
| Squash merge and keep branch           | `ggx squash --keep-branch`                 |

## Branch Behavior

1. Inspect current changes.
2. Include an optional user prompt when one is provided.
3. Generate a short branch name using OpenCode CLI with OpenCode Zen model `opencode/north-mini-code-free` and variant `none`.
4. Normalize to `type/short-kebab-name` with one of `feat`, `fix`, `refactor`, `docs`, `test`, or `chore`.
5. Generate a replacement once if the local or remote branch already exists.
6. When pending changes exist, preview all changes, generate a commit message, and show the changes and message.
7. Confirm before creating, checking out, staging, committing pending changes, and pushing the branch to `origin`.
8. When no pending changes exist, confirm before creating, checking out, and pushing the branch to `origin`.

Example output: `feat/refresh-auth-session`

## Commit Behavior

1. Fail fast if conflicts are unresolved.
2. Preview all changes, including untracked files, without changing the real index.
3. Generate a commit message from that preview.
4. Show a styled changes summary and generated message.
5. Let the user choose the commit action or cancel from an action prompt.
6. Stage all changes and commit.
7. Push automatically if upstream exists.
8. Set upstream and push if origin exists.
9. Skip push if origin is missing.

## PR Behavior

1. Detect current branch.
2. Detect base branch.
3. Require a clean worktree and an existing upstream branch.
4. Fail fast when an open pull request already exists for the current branch.
5. Generate a GitHub PR title and body from commits, changed files, diff, and optional `--closes` issue context.
6. Confirm before pushing the branch and creating the PR with `gh`.
7. Create the PR against the detected default base branch.
8. Support draft PRs.
9. Include summary and changes.

## Sync Behavior

1. Require a clean worktree.
2. Record the starting branch.
3. Fetch all remotes and prune stale remote refs.
4. Checkout the default base branch and pull with `--ff-only`.
5. Find local branches already merged into the base branch.
6. Find local branches whose upstream is gone, unless they report ahead commits.
7. Exclude the base branch and starting branch from cleanup.
8. Confirm before deleting cleanup candidates with safe `git branch -d`.
9. Return to the starting branch when sync began somewhere else.

## Merge Behavior

1. Require a clean worktree.
2. Detect the current PR or use the optional target argument.
3. Show PR number, title, URL, head/base branches, merge state, and review decision when available.
4. Confirm before running `gh pr merge --merge`.
5. Delete the branch by default, or preserve it with `--keep-branch`.
6. Pass `--admin` through to `gh pr merge` when requested.
7. Checkout the PR base branch, pull with `--ff-only`, and fetch/prune remotes after merge.

## Squash Behavior

1. Require a clean worktree.
2. Detect the current GitHub PR.
3. Show PR number, title, URL, head/base branches, merge state, and review decision when available.
4. Confirm before running `gh pr merge --squash`.
5. Delete the branch by default, or preserve it with `--keep-branch`.
6. Pass `--admin` through to `gh pr merge` when requested.
7. Checkout the PR base branch, pull with `--ff-only`, and fetch/prune remotes after squash merge.

## Admin Mode

`--admin` is only implemented for `ggx merge` and `ggx squash`. It passes `--admin` to `gh pr merge` after the normal ggx confirmation.

## One Line Pitch

ggx is a fast AI powered git workflow CLI for branches, commits, PRs, sync, and GitHub PR merge flows.

## AI Provider

ggx uses OpenCode CLI with OpenCode Zen model `opencode/north-mini-code-free` and variant `none` for branch names, commit messages, and PR copy. OpenCode Zen free models are documented as available for a limited time. North Mini Code Free has provider privacy and data-use notes; avoid sending personal or confidential data unless those terms fit your use.
