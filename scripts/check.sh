#!/usr/bin/env bash
set -e

echo "Running cargo check..."
cargo check --all-targets --all-features

echo "Running cargo fmt check..."
cargo fmt -- --check

echo "Running cargo clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "All checks passed!"
