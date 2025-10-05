RED=\033[0;31m
ORANGE_0=\033[38;5;166m
ORANGE_1=\033[38;5;208m
ORANGE_2=\033[38;5;214m
ORANGE_3=\033[38;5;220m
ORANGE_4=\033[38;5;226m

NC=\033[0m

.PHONY: help all build build-debug build-release build-fast tests unit-tests examples \
        check fmt lint clean install doc \
        build-bits test-bits build-math test-math build-log test-log build-net test-net \
        bindings wrappers vinit vcheckout vlist \
        docker-build docker-test docker-shell docker-clean docker-ci

all: build test check

help:
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
	@echo "  vinit         - Initialize submodules from fresh repo setup"
	@echo "  vlist         - List available dirs in firedancer repo"
	@echo "  vcheckout     - Checkout additional dirs from firedancer"
	@echo ""
	@echo "DOCKER (Ubuntu 24.04.2 LTS Testing):"
	@echo "  docker-build  - Build Ubuntu development container"
	@echo "  docker-test   - Run all tests in Ubuntu container"
	@echo "  docker-shell  - Open interactive shell in Ubuntu container"
	@echo "  docker-clean  - Clean Docker containers and images"
	@echo "  docker-ci     - Run full CI pipeline in Ubuntu container"

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

vinit:
	@echo "Initializing submodules from fresh repo setup..."
	@echo "Step 1: Initialize submodules..."
	@git submodule update --init --recursive
	@echo "Step 2: Configuring sparse checkout for vendor submodule..."
	@mkdir -p .git/modules/vendor/info
	@echo "src/util/*" > .git/modules/vendor/info/sparse-checkout
	@echo "src/ballet/*" >> .git/modules/vendor/info/sparse-checkout
	@echo "src/tango/*" >> .git/modules/vendor/info/sparse-checkout
	@echo "src/funk/*" >> .git/modules/vendor/info/sparse-checkout
	@echo "src/waltz/*" >> .git/modules/vendor/info/sparse-checkout
	@echo "src/disco/*" >> .git/modules/vendor/info/sparse-checkout
	@cd vendor && \
	git config core.sparseCheckout true && \
	git read-tree -m -u HEAD
	@echo "Step 3: Moving source directories to top level..."
	@cd vendor && \
	if [ -d "src" ]; then \
		for dir in src/*/; do \
			if [ -d "$$dir" ]; then \
				dirname=$$(basename "$$dir"); \
				if [ ! -d "$$dirname" ]; then \
					echo "MOVING $$dir TO $$dirname"; \
					mv "$$dir" "$$dirname"; \
				fi; \
			fi; \
		done; \
		if [ -d "src" ] && [ -z "$$(ls -A src)" ]; then \
			rmdir src; \
		fi; \
	fi
	@echo "Submodule initialization complete! Available directories:"
	@cd vendor && find . -maxdepth 1 -type d ! -name "." | sed 's|^\./||' | sort

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

docker-build:
	@echo "Building container (Ubuntu 24.04.2 LTS)..."
	@docker-compose build

docker-test:
	@echo "Running tests (Ubuntu 24.04.2 LTS)..."
	@docker-compose run --rm libfiredancer-dev make tests

docker-shell:
	@echo "Opening shell (Ubuntu 24.04.2 LTS)..."
	@docker-compose run --rm libfiredancer-dev /bin/bash

docker-clean:
	@echo "Cleaning artifacts..."
	@docker-compose down --volumes --remove-orphans
	@docker system prune -f

docker-ci:
	@echo "Running CI (Ubuntu 24.04.2 LTS)..."
	@docker-compose run --rm libfiredancer-dev make ci

docker-test-bits:
	@docker-compose run --rm libfiredancer-dev make test-bits

docker-test-math:
	@docker-compose run --rm libfiredancer-dev make test-math

docker-test-log:
	@docker-compose run --rm libfiredancer-dev make test-log

docker-test-net:
	@docker-compose run --rm libfiredancer-dev make test-net

docker-examples:
	@docker-compose run --rm libfiredancer-dev make examples
