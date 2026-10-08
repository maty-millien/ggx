# AGENTS.md

`ggx` is a Rust CLI for AI-powered git workflows.

## Rules

- After completing your task, always run `scripts/ci.sh`.
- Don't write comments in code unless asked.

## Tests

End-to-end only, in `tests/e2e/`; no `#[cfg(test)]` modules.

Every behavior change needs a new or updated test asserting what the user sees: stdout, the exact stderr error, git state, and requests sent to `gh` and providers. Answer menus via stdin (`y`, `n`, `q`, a digit, Enter) or pass `-y`. Don't test terminal-only code (arrow keys, echo handling, interactive `ggx setup`), removed features, already covered cases, clap, serde or std, or anything needing the network, a real terminal or real accounts.
