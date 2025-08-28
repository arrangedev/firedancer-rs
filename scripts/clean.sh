#!/usr/bin/env bash
set -e

echo "Cleaning build artifacts..."
cargo clean

echo "Removing temporary files..."
find . -type f -name "*.bk" -delete
find . -type f -name "*.swp" -delete
find . -type f -name "*.swo" -delete
find . -type f -name "*~" -delete

echo "Clean complete!"
