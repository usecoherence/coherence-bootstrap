# Reverse-spec: CLI layer (PR-1 inventory)

Markdown-only reverse engineering of the **`coherence-core-db` CLI** as shipped in this repo (v0 oracle / black box — **no edits under `crates/`** for this pass).

| File | Purpose |
|------|---------|
| `00-command-surface.md` | Router, full command list, unknown/version/help, core vs smoke |
| `commands/catalog.md` | `spec`, `ac` |
| `commands/verification.md` | `verify-ac`, `verify-spec`, `ac-tests` |
| `commands/db-runtime.md` | `migrate`, `db-ping`, `doctor`, `project`, `drop-isolated-test-db` |
| `commands/evidence.md` | `evidence-sample` |
| `commands/smoke-debug.md` | `m0-smoke`, `m1-spec-smoke` |
| `technical/command-router.md` | Dispatch semantics one layer down |
| `technical/world-dependencies.md` | Env, Dolt, git, FS aggregate |
| `findings.md` | Quirks and accidental candidates |

Behavioral claims here trace to `crates/coherence-core-db/src/cli.rs` and `src/commands/*.rs` as read-only sources.

Next planned passes (not this directory alone): import selected CLI SPECs/ACs into Dolt; build container/Dagger **world harness**; then black-box verify.
