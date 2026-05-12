# CLI command surface (v0 observed)

**Classification:** observed — derived from `crates/coherence-core-db/src/cli.rs` and `commands/` as of reverse-spec inventory. Not canonical catalog rows until imported.

**Binary:** `coherence-core-db` (package `coherence-core-db`, version string printed by `version` is currently literal `0.1.0` in `cli.rs`).

## Router

- argv[0]: executable name (ignored for dispatch).
- argv[1]: **top-level command** (or absent → treated as `help`).
- argv[2..]: passed to subcommand handlers for `spec`, `ac`, `ac-tests`, `verify-ac`, `verify-spec`, `evidence-sample`, `project`.

See `technical/command-router.md`.

## Command list (product grouping)

| Top-level | Exit 0 on | Notes |
|-----------|-----------|--------|
| *(default)* / `help` / `--help` / `-h` | success | Prints long help to stdout. |
| `version` / `--version` / `-V` | success | Prints fixed version line. |
| `doctor` | success | Static + env/manifest diagnostic lines; no DB ping inside doctor itself. |
| `migrate` | migrations applied | Preflight manifest, then Refinery migrations. |
| `db-ping` | server reachable | Preflight, then MySQL-protocol ping. |
| `drop-isolated-test-db` | see command doc | No-op / skip unless user-scoped Dolt. |
| `spec` | subcommand | `add` \| `list` \| `show`. |
| `ac` | subcommand | `add` \| `list` \| `show`. |
| `ac-tests` | subcommand | `materialize-rust` \| `check-rust`. |
| `verify-ac` | no failed link | Shell runner for `verified_by` codeintel links. |
| `verify-spec` | no failed link | Aggregates per-AC verification for one spec. |
| `evidence-sample` | success | Writes demo under `.coherence/runs/<run-id>/`. |
| `project` | subcommand | `catalog-preflight` \| `init` \| `reset`. |
| `m0-smoke` | smoke ok | **Internal / diagnostic candidate** — requires isolated test world, writes fixtures. |
| `m1-spec-smoke` | smoke ok | **Internal / diagnostic candidate** — same pattern, spec-module smoke. |

## Unknown command

- Any argv[1] not in the match arms: message to **stderr**, hint `coherence-core-db help`, exit **64**.

## Help / version (no separate binary flags beyond aliases)

- **Help:** same as omitting command: `help`, `--help`, `-h` → stdout help text.
- **Version:** `version`, `--version`, `-V` → single-line version to stdout.

## Core vs diagnostic / smoke

**Likely core product / operator surface:** `help`, `version`, `doctor`, `migrate`, `db-ping`, `spec`, `ac`, `verify-ac`, `verify-spec`, `ac-tests`, `evidence-sample`, `project`, `drop-isolated-test-db` (narrow ops scenario).

**Internal / smoke / legacy-style candidates:** `m0-smoke`, `m1-spec-smoke` — exist to prove vertical slices and CI-style behavior; not the primary “product command” narrative unless promoted deliberately.

## Where to read next

- `commands/catalog.md` — `spec`, `ac`
- `commands/verification.md` — `verify-ac`, `verify-spec`, `ac-tests`
- `commands/db-runtime.md` — `migrate`, `db-ping`, `doctor`, `project`, `drop-isolated-test-db`
- `commands/evidence.md` — `evidence-sample`
- `commands/smoke-debug.md` — `m0-smoke`, `m1-spec-smoke`
- `technical/command-router.md` — dispatch detail
- `technical/world-dependencies.md` — env, Dolt, git, filesystem
- `findings.md` — oddities and follow-ups
