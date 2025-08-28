#!/usr/bin/env bash
set -e

echo "Running tests..."
cargo test --all-features --workspace

echo "Running doc tests..."
cargo test --doc --workspace

echo "All tests passed!"
