#!/usr/bin/env bash

# Usage: ./scripts/test-crate.sh <crate-name> [additional-cargo-args...]

if [ $# -lt 1 ]; then
    echo "ERROR: Usage: $0 <crate-name> [additional-cargo-args...]"
    exit 1
fi

CRATE_NAME=$1
shift

if [ -f /.dockerenv ] || grep -q 'docker\|lxc' /proc/1/cgroup 2>/dev/null; then
    CARGO_TARGET="--target x86_64-unknown-linux-gnu"
else
    CARGO_TARGET=""
fi

export CFLAGS="-w"
export CXXFLAGS="-w" 
export CC_WARNINGS=0
export CARGO_TERM_COLOR=always
export RUST_BACKTRACE=1

exec cargo test --package "${CRATE_NAME}" $CARGO_TARGET "$@"