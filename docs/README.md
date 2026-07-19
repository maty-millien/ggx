# ggx

Fast AI-powered git workflows from the terminal.

`ggx` helps you turn local changes into named branches, commits, pull requests, and cleanly merged work with a small set of focused commands.

## Install

```sh
brew install maty-millien/tap/ggx
```

## Requirements

- `git`
- `gh`, authenticated with GitHub
- `opencode`, authenticated with OpenCode Zen for AI generation

## Commands

```sh
ggx branch [prompt]          # Generate a branch, commit pending changes, and push
ggx commit                   # Generate a commit message, commit, and push if origin exists
ggx pr [--draft]             # Generate and open a pull request
ggx pr --closes 123          # Include issue context in the generated PR
ggx sync                     # Sync the default branch and clean safe local branches
ggx merge                    # Merge the current PR, sync the base branch, and delete the branch
ggx merge --keep-branch      # Merge without deleting the branch
ggx squash                   # Squash merge the current PR
ggx squash --keep-branch     # Squash merge without deleting the branch
ggx --version, ggx -v        # Print the ggx version
```

Use `--admin` with `merge` or `squash` when the GitHub operation needs elevated permissions.

## Workflow

```sh
ggx branch "add billing webhook retries"
ggx commit
ggx pr --draft
ggx sync
ggx merge
```

## What It Does

- Reads your current git state and diffs.
- Asks OpenCode CLI for concise branch names, commit messages, and PR copy using OpenCode Zen model `opencode/north-mini-code-free` with variant `none`.
- Previews pending changes before confirmation, then stages and commits them during `ggx branch`.
- Shows the generated output and asks with an interactive action prompt before staging, committing, or pushing.
- Hides the cursor and suppresses accidental terminal input until an action prompt is shown.
- Fails fast when `ggx pr` finds an open pull request for the current branch.
- Requires a clean worktree before syncing or merging.
- Uses GitHub CLI for pull request creation, merge, squash, and branch cleanup.

OpenCode Zen free models are documented as available for a limited time. North Mini Code Free has provider privacy and data-use notes; avoid sending personal or confidential data unless those terms fit your use.

## License

MIT
