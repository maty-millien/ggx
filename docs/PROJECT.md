# ggx Project

ggx is a fast Rust git workflow CLI with AI generated branches, commits, PRs, squash, merge cleanup, auto push, and an easy provider API for contributors.

## Commands

| Command      | Purpose                                                                     |
| ------------ | --------------------------------------------------------------------------- |
| `ggx branch` | Generate and create a branch from current changes                           |
| `ggx commit` | Stage all changes, generate commit, commit, auto push if origin exists      |
| `ggx pr`     | Push if needed, generate PR title and body, create PR                       |
| `ggx ship`   | Smart flow: commit and push, PR                                             |
| `ggx sync`   | Fetch remotes, prune refs, sync default branch, clean stale local branches  |
| `ggx clean`  | Delete merged and stale branches                                            |
| `ggx merge`  | Merge branch or PR, delete branch by default, checkout default branch, sync |
| `ggx squash` | Squash branch commits into one clean commit                                 |

## Key Defaults

| Behavior                           | Default                       |
| ---------------------------------- | ----------------------------- |
| Commit input                       | All changes                   |
| Commit push                        | Auto push if origin exists    |
| Missing remote                     | Skip push                     |
| Branch deletion after merge        | Enabled                       |
| Remote branch deletion after merge | Enabled                       |
| Merge behavior                     | Full land flow                |
| Merge strategy                     | Normal merge only             |
| Squash flow                        | Separate `ggx squash` command |
| Destructive actions                | Confirm unless `--yes`        |
| Protected branches                 | Never delete unless `--admin` |

## Global Flags

| Flag            | Purpose                                                                        |
| --------------- | ------------------------------------------------------------------------------ |
| `--admin`       | Unlock privileged cleanup, protected branch actions, and history rewrite flows |
| `--keep-branch` | Do not delete branch after merge                                               |
| `--yes`         | Skip confirmations                                                             |
| `--dry-run`     | Preview actions                                                                |
| `--no-push`     | Disable auto push                                                              |
| `--draft`       | Create draft PR                                                                |

## Common Workflows

| Workflow                                    | Command                                    |
| ------------------------------------------- | ------------------------------------------ |
| Create branch from current changes          | `ggx branch`                               |
| Create branch from prompt                   | `ggx branch "add stripe webhook handling"` |
| Stage and commit all changes with auto push | `ggx commit`                               |
| Create PR                                   | `ggx pr`                                   |
| Create draft PR                             | `ggx pr --draft`                           |
| Branch, commit, push, PR                    | `ggx ship`                                 |
| Squash branch before PR                     | `ggx squash`                               |
| Merge PR and clean branch                   | `ggx merge`                                |
| Merge but keep branch                       | `ggx merge --keep-branch`                  |
| Full sync and cleanup                       | `ggx sync`                                 |
| Preview cleanup                             | `ggx clean --dry-run`                      |
| Admin sync and cleanup                      | `ggx sync --admin --yes`                   |

## Branch Behavior

1. Inspect current changes.
2. Infer type: `feat`, `fix`, `refactor`, `docs`, `test`, or `chore`.
3. Generate a short branch name.
4. Avoid duplicate local and remote names.
5. Create and checkout the branch.
6. Support optional prompt input.

Example output: `feat/refresh-auth-session`

## Commit Behavior

1. Fail fast if conflicts are unresolved.
2. Stage all changes, including untracked files.
3. Generate a commit message from the staged result.
4. Show a styled changes summary and generated message.
5. Let the user commit to the current branch or cancel.
6. Commit.
7. Push automatically if upstream exists.
8. Set upstream and push if origin exists.
9. Skip push if origin is missing.

## PR Behavior

1. Detect current branch.
2. Detect base branch.
3. Push branch if needed.
4. Generate PR title.
5. Generate PR body.
6. Create PR on configured forge.
7. Support draft PRs.
8. Include summary, changes, test plan, risk, and notes.

## Ship Behavior

1. Create branch if needed.
2. Commit changes if needed.
3. Push branch if needed.
4. Create or update PR.
5. Respect `--draft`, `--yes`, and `--no-push`.

## Sync Behavior

1. Fetch all remotes.
2. Prune stale remote refs.
3. Sync default branch.
4. Delete merged local branches.
5. Delete local branches whose remote is gone.
6. Keep branches with unpushed commits.
7. Keep protected branches unless `--admin`.
8. Show summary of changes.

## Clean Behavior

1. Delete merged local branches.
2. Delete local branches whose remote is gone.
3. Prune remote refs.
4. Keep branches with unpushed commits.
5. Keep protected branches unless `--admin`.
6. Support dry run preview.

## Merge Behavior

1. Fetch remote.
2. Detect current PR or target branch.
3. Run configured pre merge checks.
4. Merge using normal merge flow.
5. Delete remote branch by default.
6. Delete local branch by default.
7. Checkout default branch.
8. Sync default branch.
9. Use `--keep-branch` to preserve the branch.

Important: `ggx merge` should not have `--squash`. Squash is a separate command.

## Squash Behavior

1. Detect base branch.
2. Collect commits since base.
3. Generate one clean commit message.
4. Squash into one commit.
5. Push with lease if branch has upstream and user confirms.
6. Require `--admin` for protected or shared branch rewrites.

## Admin Mode

`--admin` unlocks advanced operations.

1. Protected branch cleanup after confirmation.
2. Remote branch deletion in cleanup flows.
3. Pushed branch history rewrite.
4. Force with lease.
5. Repo wide stale branch cleanup.
6. Protected merge flows.

Admin still requires confirmation unless `--yes` is passed.

## Provider API

ggx supports native providers and community providers.

Provider types:

| Provider         | Purpose                                          |
| ---------------- | ------------------------------------------------ |
| `openai`         | Built in OpenAI support                          |
| `codex`          | Codex compatible mode where officially supported |
| `anthropic`      | Optional hosted model support                    |
| `ollama`         | Local model support                              |
| `lmstudio`       | Local desktop model support                      |
| `custom-process` | Any provider over JSON stdio                     |
| `mock`           | Tests and demos                                  |

Provider capabilities:

| Capability      | Purpose                    |
| --------------- | -------------------------- |
| `commit`        | Generate commit messages   |
| `branch`        | Generate branch names      |
| `pull_request`  | Generate PR title and body |
| `diff_summary`  | Summarize diffs            |
| `conflict_help` | Explain conflicts          |

Provider interface goals:

1. Stable versioned protocol.
2. Capability discovery.
3. Native Rust provider support.
4. External process provider support.
5. Clear request and response schemas.
6. Graceful fallback when a provider lacks a capability.
7. Simple contributor path for adding new providers.

## Forge Providers

| Forge     | Features                           |
| --------- | ---------------------------------- |
| GitHub    | PR, draft PR, merge, branch delete |
| GitLab    | MR, draft MR, merge, branch delete |
| Forgejo   | PR, merge, branch delete           |
| Gitea     | PR, merge, branch delete           |
| Bitbucket | Later                              |

## One Line Pitch

ggx is a fast AI powered git workflow CLI for branches, commits, PRs, squash, automatic pushing, clean merges, and local repos that stay synced with remote.
