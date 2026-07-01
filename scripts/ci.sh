#!/usr/bin/env sh
set -eu

echo "Running cargo fmt"
cargo fmt

echo "Running cargo clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "Running cargo test"
cargo test --all-targets --all-features

echo "Running cargo build"
cargo build --locked

echo "Running cargo audit"
cargo audit
