#!/usr/bin/env sh
set -eu

# Files that only orchestrate git, gh, curl, provider CLIs, or the terminal.
# They are excluded from coverage; keep logic out of them.
IO_FILES='src/(main|cli|config|tui)\.rs$|src/ai/(mod|claude|codex|copilot)\.rs$|src/vcs/(git|github)\.rs$|src/commands/(setup|update)\.rs$|src/commands/[a-z]+/(run|context|mod)\.rs$|src/(ai|commands|vcs)/mod\.rs$'

echo "Running cargo fmt"
cargo fmt

echo "Running cargo clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "Running cargo test"
cargo test --all-targets --all-features

echo "Running cargo llvm-cov"
cargo llvm-cov --all-targets --all-features --ignore-filename-regex "$IO_FILES" --fail-under-lines 95 --show-missing-lines

echo "Running cargo build"
cargo build --locked
