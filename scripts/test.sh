#!/usr/bin/env bash
set -e

if [ -f /.dockerenv ] || grep -q 'docker\|lxc' /proc/1/cgroup 2>/dev/null; then
    cargo test --all-features --workspace --target x86_64-unknown-linux-gnu
elif [ "$(uname)" = "Darwin" ]; then
    cargo test --all-features --workspace --target x86_64-unknown-linux-gnu
else
    echo "Running tests on $(uname -s) $(uname -m)..."
    cargo test --all-features --workspace
fi
