# AGENTS.md

This project is `ggx`, a Rust CLI for fast AI-powered git workflows. The main code lives in `src/`, user-facing docs are in `docs/`, and helper scripts are in `scripts/`.

Always keep documentation up to date as part of any change or implementation plan.

## Tests

Tests live in `#[cfg(test)]` modules next to the code. When you add or change behavior, add or update tests for it.

The code is split so that coverage means something: logic lives in pure files (`validation.rs`, `context/diff.rs`, `generation.rs`, `tui/format.rs`, `*/parse.rs`, `*/summary.rs`, `sync/candidates.rs`, `setup/select.rs`, `update/check.rs`, `config/file.rs`, `vcs/changes.rs`), and the files matched by `IO_FILES` in `scripts/ci.sh` only orchestrate git, gh, curl, the provider CLIs, or the terminal. `scripts/ci.sh` requires 95% line coverage on the pure files; a whole untested branch fails it, a stray closing brace or an uncalled test closure does not. When you add logic, put it in a pure file, or extract it into one, so it is measured. Do not add abstractions to the I/O files just to make them testable.

Never fight the coverage tool. If a line stays red because of how llvm-cov counts rather than because of a missing test (an implicit else brace, a closure a test deliberately never calls, a generic function instantiated per test, a defensive branch that cannot happen, an I/O error path), leave it red. Do not rewrite production code, replace generics with `dyn`, remove defensive branches, or contort tests to move that line. The 5% margin exists for exactly those lines. Coverage needs `cargo install cargo-llvm-cov` and `rustup component add llvm-tools-preview`.

Write tests that pin down real behavior: the pure logic behind each command (parsing, validation, prompt rendering, formatting) and each error branch with its exact message. Do not write tests that:

- check that a removed feature or argument is still gone;
- copy the implementation into the test (for example asserting a full argument list constant by constant);
- repeat a case already covered through a different input;
- exercise clap, serde, or the standard library instead of this project's logic;
- need a real terminal, network, or the `git`, `gh`, `curl`, or provider CLIs.

After any code or documentation modification, run exactly:

```sh
scripts/ci.sh
```

Do not run any other build, test, lint, format, or check command unless the user explicitly asks for it.
