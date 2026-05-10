.PHONY: tool check test test-isolated smoke smoke-isolated \
	test-world-reset fmt clippy install-local install-local-force install-local-check print-bin-name

BIN_NAME ?= coherence-core-db
INSTALL_ROOT ?= $(HOME)/.local
INSTALL_BIN_DIR := $(INSTALL_ROOT)/bin
CRATE_PATH ?= crates/coherence-core-db

tool:
	@./scripts/tool $(filter-out $@,$(MAKECMDGOALS))

check:
	@$(MAKE) tool run

# Isolated-by-default: workspace tests never run without explicit test-world profile.
test: test-isolated

test-isolated:
	@./scripts/with-isolated-test-profile cargo test --workspace

# Mutating smoke against Dolt requires the same profile as unit integration tests (see test_world_guard).
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

install-local: install-local-check
	@cargo install --path "$(CRATE_PATH)" --root "$(INSTALL_ROOT)" --locked
	@echo "Installed $(BIN_NAME) to $(INSTALL_BIN_DIR)/$(BIN_NAME)"

install-local-force:
	@mkdir -p "$(INSTALL_BIN_DIR)"
	@cargo install --path "$(CRATE_PATH)" --root "$(INSTALL_ROOT)" --locked --force
	@echo "Installed (forced) $(BIN_NAME) to $(INSTALL_BIN_DIR)/$(BIN_NAME)"

%:
	@:
