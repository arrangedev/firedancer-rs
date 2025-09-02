.PHONY: help all build build-debug build-release build-fast tests unit-tests examples \
        check fmt lint clean install doc \
        build-bits test-bits build-math test-math build-log test-log build-net test-net \
        bindings wrappers vcheckout vlist

all: build test check

help:
	@echo "firedancer-rs"
	@echo ""
	@echo "BUILD:"
	@echo "  build         - Build everything in fast mode"
	@echo "  build-debug   - Build everything in debug mode"
	@echo "  build-release - Build everything in release mode"
	@echo "  build-fast    - Build everything in fast mode"
	@echo ""
	@echo "TESTS:"
	@echo "  tests         - Run all tests (unit + doc)"
	@echo "  unit-tests    - Run unit tests only"
	@echo ""
	@echo "CONVENTION:"
	@echo "  check         - Run all checks (fmt + clippy + build check)"
	@echo "  check-fmt     - Check formatting"
	@echo "  check-clippy  - Run linter"
	@echo ""
	@echo "DOCS:"
	@echo "  doc           - Generate docs"
	@echo ""
	@echo "UTILS:"
	@echo "  clean         - Clean build artifacts/tempfiles"
	@echo "  examples      - Run all examples"
	@echo ""
	@echo "TARGETS:"
	@echo "  build-bits    - Build fd_bits and fd_bits_sys"
	@echo "  test-bits     - Test fd_bits and fd_bits_sys"
	@echo "  build-math    - Build fd_math and fd_math_sys"
	@echo "  test-math     - Test fd_math and fd_math_sys"
	@echo "  build-log     - Build fd_log and fd_log_sys"
	@echo "  test-log      - Test fd_log and fd_log_sys"
	@echo "  build-net     - Build fd_net and fd_net_sys"
	@echo "  test-net      - Test fd_net and fd_net_sys"
	@echo ""
	@echo "CRATE TYPE:"
	@echo "  bindings      - Build all bindings (fd_*_sys)"
	@echo "  wrappers      - Build all wrapper crates (fd_*)"
	@echo ""
	@echo "VENDOR:"
	@echo "  vlist         - List available dirs in firedancer repo"
	@echo "  vcheckout     - Checkout additional dirs from firedancer"

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
	@cargo build --package fd_bits_sys --all-features
	@cargo build --package fd_bits --all-features
	@echo "Done."

test-bits:
	@cargo test --package fd_bits_sys --all-features
	@cargo test --package fd_bits --all-features
	@echo "Done."

build-math:
	@cargo build --package fd_math_sys --all-features
	@cargo build --package fd_math --all-features
	@echo "Done."

test-math:
	@cargo test --package fd_math_sys --all-features
	@cargo test --package fd_math --all-features
	@echo "Done."

build-log:
	@cargo build --package fd_log_sys --all-features
	@cargo build --package fd_log --all-features
	@echo "Done."

test-log:
	@cargo test --package fd_log_sys --all-features
	@cargo test --package fd_log --all-features
	@echo "Done."

build-net:
	@cargo build --package fd_net_sys --all-features
	@cargo build --package fd_net --all-features
	@echo "Done."

test-net:
	@cargo test --package fd_net_sys --all-features
	@cargo test --package fd_net --all-features
	@echo "Done."

bindings:
	@cargo build --package fd_bits_sys --all-features
	@cargo build --package fd_math_sys --all-features
	@cargo build --package fd_log_sys --all-features
	@cargo build --package fd_net_sys --all-features
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

vlist:
	@echo "CHECKED OUT:"
	@cd vendor && find . -maxdepth 1 -type d ! -name "." | sed 's|^\./||' | sort
	@echo ""
	@echo "AVAILABLE (from firedancer/src/):"
	@cd vendor && git ls-tree -d --name-only HEAD:src/ | grep -v "^util$$\|^ballet$$\|^tango$$" | sort
	@echo ""
	@echo "- checkout new dirs, with: make vcheckout DIRS=\"dirname1 dirname2\""

vcheckout:
	@if [ -z "$(DIRS)" ]; then \
		echo "ERROR: Please specify dirs using DIRS=\"name1 name2\""; \
		echo "Ex: 'make vcheckout DIRS=\"flamenco disco\"'"; \
		echo "USE 'make vlist' TO SEE AVAILABLE"; \
		exit 1; \
	fi
	@echo "CHECKING OUT: $(DIRS)"
	@cd vendor && \
	for dir in $(DIRS); do \
		echo "src/$$dir/*" >> ../.git/modules/vendor/info/sparse-checkout; \
	done && \
	git read-tree -m -u HEAD && \
	for dir in $(DIRS); do \
		if [ -d "src/$$dir" ]; then \
			echo "MOVING src/$$dir TO $$dir"; \
			mv "src/$$dir" .; \
		else \
			echo "WARNING: Directory src/$$dir not found after checkout"; \
		fi; \
	done && \
	if [ -d "src" ] && [ -z "$$(ls -A src)" ]; then \
		rmdir src; \
	fi
	@echo "Done. Updated dirs:"
	@cd vendor && find . -maxdepth 1 -type d ! -name "." | sed 's|^\./||' | sort
