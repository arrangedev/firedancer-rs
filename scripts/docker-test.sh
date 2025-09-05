#!/usr/bin/env bash

# Usage: ./scripts/docker-test-crate.sh <crate-name> [additional-cargo-args...]

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

print_error() {
    echo -e "${RED}${BOLD}ERROR: $1${NC}" >&2
}

print_success() {
    echo -e "${GREEN}${BOLD}SUCCESS: $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}${BOLD}WARNING: $1${NC}"
}

print_status() {
    local color=$1
    local message=$2
    echo -e "${color}${BOLD}=== ${message} ===${NC}"
}

if [ $# -lt 1 ]; then
    print_error "Usage: $0 <crate-name> [additional-cargo-args...]"
    exit 1
fi

CRATE_NAME=$1
shift

if ! docker image inspect libfiredancer-test:latest >/dev/null 2>&1; then
    print_status "$BLUE" "Building..."
    docker build -t libfiredancer-test:latest .
fi

print_status "$BLUE" "Running tests: ${CRATE_NAME}"
echo

TEMP_LOG=$(mktemp)
trap "rm -f $TEMP_LOG" EXIT

set +e
docker run --rm -v $(pwd):/workspace libfiredancer-test:latest bash -c "
    cd /workspace && 
    export RUST_BACKTRACE=1
    export CARGO_TERM_COLOR=always
    cargo test --package '${CRATE_NAME}' $* 2>&1
    echo \"CARGO_EXIT_CODE:\$?\"
" > "$TEMP_LOG"

DOCKER_EXIT_CODE=$?
CARGO_EXIT_CODE=$(grep "CARGO_EXIT_CODE:" "$TEMP_LOG" | cut -d: -f2 || echo "$DOCKER_EXIT_CODE")

sed -i '/CARGO_EXIT_CODE:/d' "$TEMP_LOG" 2>/dev/null || true

if [ "$CARGO_EXIT_CODE" = "0" ]; then
    print_success "All tests passed: ${CRATE_NAME}!"
    echo
    print_status "$GREEN" "Summary"
    grep -E "(test result:|running [0-9]+ tests)" "$TEMP_LOG" | tail -2 || echo "Tests completed"
else
    print_error "Build or test failed: ${CRATE_NAME} (exit code: $CARGO_EXIT_CODE)"
    echo
    
    print_status "$RED" "Errors"
    
    if grep -q "error occurred in cc-rs" "$TEMP_LOG"; then
        grep -A 10 "error occurred in cc-rs" "$TEMP_LOG" | head -15
        echo
        
    elif grep -q "error\[E[0-9]*\]" "$TEMP_LOG"; then
        grep -A 5 -B 2 "error\[E[0-9]*\]" "$TEMP_LOG" | head -20
        
    elif grep -q -E "(ld:|cannot find.*\.so|undefined reference)" "$TEMP_LOG"; then
        grep -A 3 -B 1 -E "(ld:|cannot find.*\.so|undefined reference)" "$TEMP_LOG" | head -20
        
    elif grep -q "test result: FAILED" "$TEMP_LOG"; then
        grep -A 10 -B 5 "FAILED" "$TEMP_LOG" | head -30
        
    elif grep -q "panicked" "$TEMP_LOG"; then
        grep -A 5 -B 2 "panicked" "$TEMP_LOG" | head -20
        
    elif grep -q "^error:" "$TEMP_LOG"; then
        grep -A 3 -B 1 "^error:" "$TEMP_LOG" | head -20
        
    else
        tail -30 "$TEMP_LOG" | head -20
    fi
    
    echo
    print_warning "Output saved to: $TEMP_LOG"
fi

exit $CARGO_EXIT_CODE