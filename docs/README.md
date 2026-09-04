<div align="center">

# ggx

### Ship git work without writing the branch, commit, or PR copy yourself.

`branch` · `commit` · `pull request` · `merge`

</div>

## Install

```sh
curl -fsSL https://github.com/maty-millien/ggx/releases/latest/download/ggx-installer.sh | sh
ggx setup
```

Works on macOS and Linux. You'll need `git`, GitHub CLI (`gh`), and either Codex or Claude.

<details>
<summary>Setup and updates</summary>

Authenticate `gh` and your chosen AI CLI before running `ggx setup`. Your provider choice applies to every repository. Run setup again at any time to switch.

`ggx` checks for stable updates once a day in the background. To update now, run:

```sh
ggx update
```

</details>

## The workflow

```text
  local changes
       │
       ▼
  ggx branch ─── name branch, commit, push
       │
       ▼
  ggx commit ─── commit more work, push
       │
       ▼
  ggx pr ─────── open a pull request
       │
       ▼
  ggx merge ──── merge, sync, clean up
```

```sh
ggx branch "add billing webhook retries"
ggx commit
ggx pr --base dev --draft
ggx merge
```

Run `ggx sync` whenever you want to update the default branch and clean safe local branches.

## Command guide

| I want to… | Run |
|---|---|
| Choose Codex or Claude | `ggx setup` |
| Start work from pending changes | `ggx branch [prompt]` |
| Commit and push current changes | `ggx commit` |
| Open a pull request | `ggx pr [--draft]` |
| Sync and clean local branches | `ggx sync` |
| Merge the current pull request | `ggx merge` |
| Squash-merge the current pull request | `ggx squash` |
| Install the latest stable release | `ggx update` |

<details>
<summary>Useful pull request and merge options</summary>

```sh
ggx pr --base dev       # Open against dev
ggx pr --closes 123     # Use issue 123 as context
ggx merge --keep-branch # Keep the branch after merging
ggx squash --keep-branch
```

Add `--admin` to `merge` or `squash` when GitHub requires elevated permissions.

</details>

<details>
<summary>What happens behind the scenes?</summary>

`ggx` reads the current git state and diff, then asks your chosen AI CLI to generate the relevant branch name, commit message, or pull request copy. You review the result before it stages, commits, or pushes anything.

It uses:

- `gpt-5.6-luna` with no reasoning effort through Codex
- `haiku` through Claude
- GitHub CLI for pull requests, merges, and branch cleanup

Syncing and merging require a clean worktree. Pull request creation stops if the current branch already has an open pull request. Closed and merged pull requests do not prevent creating a new one from the same branch.

</details>

## License

MIT
