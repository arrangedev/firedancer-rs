#!/usr/bin/env bash
set -e

echo "Building release version..."
cargo build --release --all-features

echo "Build complete! Binaries are in target/release/"
