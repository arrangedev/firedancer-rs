.PHONY: help all build build-debug build-release build-fast tests unit-tests examples \
        check fmt lint clean install doc \
        build-bits test-bits build-math test-math build-log test-log build-net test-net \
        bindings wrappers

all: build test check

help:
	@echo "firedancer-rs"
	@echo ""
	@echo "Build targets:"
	@echo "  build         - Build everything in fast mode"
	@echo "  build-debug   - Build everything in debug mode"
	@echo "  build-release - Build everything in release mode"
	@echo "  build-fast    - Build everything in fast mode"
	@echo ""
	@echo "Test targets:"
	@echo "  tests         - Run all tests (unit + doc)"
	@echo "  unit-tests    - Run unit tests only"
	@echo ""
	@echo "Quality targets:"
	@echo "  check         - Run all checks (fmt + clippy + build check)"
	@echo "  check-fmt     - Check formatting"
	@echo "  check-clippy  - Run linter"
	@echo ""
	@echo "Docs:"
	@echo "  doc           - Generate docs"
	@echo ""
	@echo "Util:"
	@echo "  clean         - Clean build artifacts/tempfiles"
	@echo "  examples      - Run all examples"
	@echo ""
	@echo "Crate Targets:"
	@echo "  build-bits    - Build fd_bits and libfd_bits_sys"
	@echo "  test-bits     - Test fd_bits and libfd_bits_sys"
	@echo "  build-math    - Build fd_math and libfd_math_sys"
	@echo "  test-math     - Test fd_math and libfd_math_sys"
	@echo "  build-log     - Build fd_log and libfd_log_sys"
	@echo "  test-log      - Test fd_log and libfd_log_sys"
	@echo "  build-net     - Build fd_net and libfd_net_sys"
	@echo "  test-net      - Test fd_net and libfd_net_sys"
	@echo ""
	@echo "By Crate Type:"
	@echo "  bindings    - Build all bindings (libfd_*_sys)"
	@echo "  wrappers   - Build all wrapper crates (fd_*)"

build: build-fast

build-debug:
	@cargo build --workspace --all-features
	@echo "Done."

build-release:
	@./scripts/build-release.sh

build-fast:
	@cargo build --workspace --all-features --profile fast
	@echo "Done."

tests:
	@./scripts/test.sh

unit-tests:
	@cargo test --workspace --all-features
	@echo "Done."

check:
	@./scripts/check.sh

fmt:
	@cargo +nightly fmt --all -- --check
	@echo "Done."

lint:
	@cargo clippy --workspace --all-targets --all-features -- -D warnings
	@echo "Done."

doc:
	@cargo doc --workspace --all-features --no-deps
	@echo "Docs generated in target/doc/"

clean:
	@./scripts/clean.sh

examples:
	@cargo run --example usage --package fd_log
	@cargo run --example usage --package fd_math
	@cargo run --example usage --package fd_net
	@echo "Done."

build-bits:
	@cargo build --package libfd_bits_sys --all-features
	@cargo build --package fd_bits --all-features
	@echo "Done."

test-bits:
	@cargo test --package libfd_bits_sys --all-features
	@cargo test --package fd_bits --all-features
	@echo "Done."

build-math:
	@cargo build --package libfd_math_sys --all-features
	@cargo build --package fd_math --all-features
	@echo "Done."

test-math:
	@cargo test --package libfd_math_sys --all-features
	@cargo test --package fd_math --all-features
	@echo "Done."

build-log:
	@cargo build --package libfd_log_sys --all-features
	@cargo build --package fd_log --all-features
	@echo "Done."

test-log:
	@cargo test --package libfd_log_sys --all-features
	@cargo test --package fd_log --all-features
	@echo "Done."

build-net:
	@cargo build --package libfd_net_sys --all-features
	@cargo build --package fd_net --all-features
	@echo "Done."

test-net:
	@cargo test --package libfd_net_sys --all-features
	@cargo test --package fd_net --all-features
	@echo "Done."

bindings:
	@cargo build --package libfd_bits_sys --all-features
	@cargo build --package libfd_math_sys --all-features
	@cargo build --package libfd_log_sys --all-features
	@cargo build --package libfd_net_sys --all-features
	@echo "Done."

wrappers:
	@cargo build --package fd_bits --all-features
	@cargo build --package fd_math --all-features
	@cargo build --package fd_log --all-features
	@cargo build --package fd_net --all-features
	@echo "Done."

dev-setup: clean build test
	@echo "Done."

ci: check test build-release
	@echo "Done."

install:
	@cargo install --path . --force
	@echo "Done."
