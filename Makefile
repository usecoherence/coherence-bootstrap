.PHONY: tool check test fmt clippy

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

%:
	@:
