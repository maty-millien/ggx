# ggx Project

ggx is a fast Rust git workflow CLI with AI generated branches, commits, and PR copy, plus GitHub PR merge and squash flows.

## Commands

| Command      | Purpose                                                                     |
| ------------ | --------------------------------------------------------------------------- |
| `ggx setup`  | Choose and save the AI provider                                             |
| `ggx branch` | Generate a branch, commit pending changes, and push                         |
| `ggx commit` | Preview all changes, confirm, commit, auto push if origin exists            |
| `ggx pr`     | Prepare pending work, push, and create a GitHub pull request                |
| `ggx sync`   | Sync the default branch and clean safe local branches                       |
| `ggx update` | Check for and install the latest stable release                             |
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
| AI provider scope                   | One selection per user           |
| Commands before provider setup      | Fail with setup instructions     |

## Distribution

Cargo Dist publishes native arm64 and x86_64 binaries for macOS and Linux with a shell installer. Global installations check for stable updates silently once a day, and `ggx update` runs the updater immediately.

A successful CI run on `main` compares the version in `Cargo.toml` with existing release tags. A new version automatically starts the release workflow, which creates the tag, GitHub Release, binaries, installer, and updater.

## Command Flags

| Flag              | Purpose                                                                |
| ----------------- | ---------------------------------------------------------------------- |
| `--draft`         | Create a draft PR with `ggx pr`                                        |
| `--closes`        | Include issue context in a generated PR body with `ggx pr`             |
| `--base`          | Target a specific PR base branch instead of the repository default      |
| `--keep-branch`   | Do not delete the branch after `ggx merge` or `ggx squash`             |
| `--admin`         | Pass admin privileges to `gh pr merge` for `ggx merge` or `ggx squash` |
| `--version`, `-v` | Print the ggx version                                                  |

## Common Workflows

| Workflow                               | Command                                    |
| -------------------------------------- | ------------------------------------------ |
| Choose or change the AI provider       | `ggx setup`                                |
| Create branch from current changes     | `ggx branch`                               |
| Create branch from prompt              | `ggx branch "add stripe webhook handling"` |
| Preview, stage, and commit all changes | `ggx commit`                               |
| Create PR                              | `ggx pr`                                   |
| Create draft PR                        | `ggx pr --draft`                           |
| Include issue context in PR text       | `ggx pr --closes 123`                      |
| Create a PR targeting `dev`            | `ggx pr --base dev`                        |
| Sync base branch and clean locals      | `ggx sync`                                 |
| Install the latest stable release      | `ggx update`                               |
| Merge PR and clean branch              | `ggx merge`                                |
| Merge but keep branch                  | `ggx merge --keep-branch`                  |
| Squash merge current PR                | `ggx squash`                               |
| Squash merge and keep branch           | `ggx squash --keep-branch`                 |

## Branch Behavior

1. Inspect current changes.
2. Include an optional user prompt when one is provided.
3. Generate a short branch name and pending commit message in one request to the configured AI provider.
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
2. Use `--base` when provided, otherwise detect the repository default base branch.
3. When run from the selected base with pending changes, generate a new branch, commit, and PR together.
4. When run from a feature branch with pending changes, generate the commit and PR together.
5. When run from a clean feature branch, generate only the PR.
6. Fail fast when there are no changes or an open pull request already exists for the current feature branch.
7. Generate every required artifact in one Codex CLI request, with one retry for invalid output or an existing branch name.
8. Show all generated output and confirm once before creating a branch, committing, pushing, or creating the PR.
9. Create the PR against the selected base and support draft PRs and `--closes` issue context.

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

## Update Behavior

1. Installed copies check for a stable release silently in the background at most once every 24 hours.
2. Automatic update failures never interrupt the active command.
3. `ggx update` runs the installed `ggx-update` helper immediately and waits for it to finish.
4. Manual updates report the installed version or fail with the updater error.

## Merge Behavior

1. Require a clean worktree.
2. Detect the current PR.
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

`ggx setup` requires an interactive terminal and lets the user choose Codex or Claude. It checks that the selected CLI is installed before saving the choice in `$XDG_CONFIG_HOME/ggx/config.json`, or `$HOME/.config/ggx/config.json` when `XDG_CONFIG_HOME` is unset. Running setup again changes the provider for every repository.

Every command except setup and version requires a valid provider configuration. Missing or invalid configuration tells the user to run `ggx setup`.

Codex uses `gpt-5.6-luna` with low reasoning effort. Claude uses the `haiku` alias without an effort flag because Haiku does not support configurable effort. Each command requests all of its generated artifacts together. ggx removes one surrounding JSON markdown fence from any provider before parsing and validation.
