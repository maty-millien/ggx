# ggx

Fast AI-powered git workflows from the terminal.

`ggx` helps you turn local changes into named branches, commits, pull requests, and cleanly merged work with a small set of focused commands.

## Install

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/maty-millien/ggx/releases/latest/download/ggx-installer.sh | sh
```

The installer supports Apple Silicon and Intel Macs, plus arm64 and x86_64 Linux.

`ggx` checks for stable updates silently in the background once a day. Run `ggx update` to check and update immediately.

## Requirements

- `git`
- `gh`, authenticated with GitHub
- `codex`, authenticated for AI generation

## Commands

```sh
ggx branch [prompt]          # Generate a branch, commit pending changes, and push
ggx commit                   # Generate a commit message, commit, and push if origin exists
ggx pr [--draft]             # Commit pending work, push, and open a pull request
ggx pr --base dev            # Create the pull request against dev
ggx pr --closes 123          # Include issue context in the generated PR
ggx sync                     # Sync the default branch and clean safe local branches
ggx update                   # Check for and install a stable update
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
ggx pr --base dev --draft
ggx sync
ggx merge
```

## What It Does

- Reads your current git state and diffs.
- Asks Codex CLI for concise branch names, commit messages, and PR copy using `gpt-5.6-luna` with low reasoning effort, grouping each command's output into one request.
- Previews pending changes before confirmation, then stages and commits them during `ggx branch`.
- Shows the generated output and asks with an interactive action prompt before staging, committing, or pushing.
- Hides the cursor and suppresses accidental terminal input until an action prompt is shown.
- Turns pending work on the selected base branch into a new branch, commit, push, and pull request with one confirmation.
- Commits pending changes on an existing feature branch before creating its pull request.
- Fails fast when `ggx pr` finds an open pull request for the current branch.
- Requires a clean worktree before syncing or merging.
- Uses GitHub CLI for pull request creation, merge, squash, and branch cleanup.

## License

MIT
