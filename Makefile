.PHONY: tool check test fmt clippy install-local install-local-force install-local-check print-bin-name

BIN_NAME ?= coherence-core-db
INSTALL_ROOT ?= $(HOME)/.local
INSTALL_BIN_DIR := $(INSTALL_ROOT)/bin
CRATE_PATH ?= crates/coherence-core-db

tool:
	@./scripts/tool $(filter-out $@,$(MAKECMDGOALS))

check:
	@$(MAKE) tool run

test:
	@cargo test --workspace

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
