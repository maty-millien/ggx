#!/usr/bin/env sh
set -eu

echo "Running cargo fmt"
cargo fmt

echo "Running cargo clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "Running cargo llvm-cov"
cargo llvm-cov --all-targets --all-features --show-missing-lines

echo "Running cargo build"
cargo build --locked
