# AGENTS.md

`ggx` is a Rust CLI for AI-powered git workflows. Code is in `src/`, docs in `docs/`, scripts in `scripts/`. Keep docs up to date with every change.

## Tests

Tests are end-to-end only, in `tests/e2e/`; don't add `#[cfg(test)]` modules. Each test runs the `ggx` binary in a temporary home with a real git repo and a local bare `origin`. `gh`, `curl`, `codex`, `claude` and `security` are fake scripts on `PATH` that replay responses queued with `Env::respond` (or `Env::reply` for Codex) and record their arguments.

For every behavior change, add or update a test asserting what the user sees: stdout, the exact stderr error, the git state, and the requests sent to `gh` and the providers. Answer menus through stdin (`y`, `n`, `q`, a digit, Enter), or pass `-y`. Leave terminal-only code (arrow keys, echo handling, interactive `ggx setup`) untested.

`scripts/ci.sh` lists uncovered lines, with no threshold. Check them in the files you touched: each one is either a missing test or dead code to remove. Ignore lines red only because of how llvm-cov counts or because a test can't trigger them, and never contort code or tests to cover them. Coverage needs `cargo-llvm-cov` and `llvm-tools-preview`.

Don't write tests that check a removed feature is gone, repeat a covered case, exercise clap, serde or std, or need the network, a real terminal or real accounts.

After making changes run:

```sh
scripts/ci.sh
```
