.PHONY: tool check build test test-isolated smoke smoke-isolated \
	test-world-reset fmt clippy install-local install-local-force install-local-check print-bin-name \
	cleanup-user-scoped demo-container-build demo-container-smoke demo-container-shell

BIN_NAME ?= coherence-bootstrap
INSTALL_ROOT ?= $(HOME)/.local
INSTALL_BIN_DIR := $(INSTALL_ROOT)/bin
CRATE_PATH ?= .
DEMO_IMAGE ?= coherence-bootstrap-demo:local
DEMO_WORKSPACE ?= $(CURDIR)

tool:
	@./scripts/tool $(filter-out $@,$(MAKECMDGOALS))

check:
	@$(MAKE) tool run

build:
	@cargo build --workspace --locked

# Isolated-by-default: workspace tests never run without explicit test-world profile.
test: test-isolated

test-isolated:
	@./scripts/with-isolated-test-profile cargo test --workspace $(CARGO_TEST_ARGS)

# Mutating smoke uses with-isolated-test-profile (COHERENCE_DB_PROFILE=test, COHERENCE_ENV=test); guard in test_world_guard.rs.
smoke-isolated smoke:
	@./scripts/with-isolated-test-profile cargo run -p coherence-core-db -- m0-smoke
	@./scripts/with-isolated-test-profile cargo run -p coherence-core-db -- m1-spec-smoke

test-world-reset:
	@COHERENCE_DB_PROFILE=test ./scripts/test-world-reset

fmt:
	@cargo fmt --all

clippy:
	@cargo clippy --workspace --all-targets -- -D warnings

print-bin-name:
	@echo $(BIN_NAME)

install-local-check:
	@mkdir -p "$(INSTALL_BIN_DIR)"
	@if [ -e "$(INSTALL_BIN_DIR)/$(BIN_NAME)" ]; then \
		echo "ERROR: $(INSTALL_BIN_DIR)/$(BIN_NAME) already exists."; \
		echo "Refusing to overwrite to avoid collisions."; \
		echo "Use 'make install-local-force' to overwrite intentionally."; \
		exit 1; \
	fi
	@if command -v "$(BIN_NAME)" >/dev/null 2>&1; then \
		echo "WARNING: '$(BIN_NAME)' is already on PATH at $$(command -v "$(BIN_NAME)")"; \
	fi

install-local: install-local-check build
	@cargo install --path "$(CRATE_PATH)" --root "$(INSTALL_ROOT)" --locked
	@echo "Installed $(BIN_NAME) to $(INSTALL_BIN_DIR)/$(BIN_NAME)"

install-local-force: build
	@mkdir -p "$(INSTALL_BIN_DIR)"
	@cargo install --path "$(CRATE_PATH)" --root "$(INSTALL_ROOT)" --locked --force
	@echo "Installed (forced) $(BIN_NAME) to $(INSTALL_BIN_DIR)/$(BIN_NAME)"

cleanup-user-scoped:
	@echo "Stopping all dolt sql-server processes..."
	@for pid in $$(pgrep -f "dolt sql-server" 2>/dev/null); do \
		echo "  Killing PID $$pid"; \
		kill $$pid 2>/dev/null || true; \
	done
	@sleep 1
	@echo "Cleaning user-scoped data-dir..."
	@rm -rf /home/br11k/.local/share/coherence/db/*
	@echo "Cleaning user-scoped runtime dir..."
	@rm -rf /run/user/1000/coherence/*
	@echo "cleanup-user-scoped: done"

demo-container-build:
	@docker build -t "$(DEMO_IMAGE)" .

demo-container-smoke: demo-container-build
	@tmp_dir=$$(mktemp -d /tmp/coherence-bootstrap-demo-smoke.XXXXXX); \
		docker run --rm -v "$$tmp_dir:/workspace" "$(DEMO_IMAGE)" coherence-demo-smoke

demo-container-shell: demo-container-build
	@docker run --rm -it -v "$(DEMO_WORKSPACE):/workspace" "$(DEMO_IMAGE)" bash

%:
	@:
