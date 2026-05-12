# PR-2 — Import CLI command surface into catalog

**Goal:** one SPEC and five ACs in the Coherence catalog, matching the markdown inventory under `../cli/`, using **only** the shipped `coherence-core-db spec` / `ac` CLI (no production code edits).

## Stable ids

| Kind | id | slug |
|------|-----|------|
| SPEC | `CLI-COMMAND-SURFACE-SPEC` | `coredb-cli-command-surface` |
| AC | `AC-CLI-SURFACE-HELP` | `help-command-exposes-surface` |
| AC | `AC-CLI-SURFACE-VERSION` | `version-command-prints-version` |
| AC | `AC-CLI-SURFACE-UNKNOWN` | `unknown-command-exits-64` |
| AC | `AC-CLI-SURFACE-ROUTER` | `top-level-command-groups-are-routed` |
| AC | `AC-CLI-SURFACE-SMOKE` | `smoke-commands-are-internal-candidates` |

## How to run

From repository root, with an isolated disposable catalog (ADR-0004):

```bash
./scripts/with-isolated-test-profile ./.coherence/reverse-spec/pr2-catalog-import/import-cli-command-surface.sh
```

Optional: use a pre-built binary:

```bash
export COHERENCE_CORE_DB_BIN="$PWD/target/debug/coherence-core-db"
./scripts/with-isolated-test-profile ./.coherence/reverse-spec/pr2-catalog-import/import-cli-command-surface.sh
```

## Verify

```bash
./scripts/with-isolated-test-profile cargo run -q -p coherence-core-db -- spec show CLI-COMMAND-SURFACE-SPEC
./scripts/with-isolated-test-profile cargo run -q -p coherence-core-db -- ac list --spec-id CLI-COMMAND-SURFACE-SPEC
```

## Idempotency / re-runs

The shipped `spec add` / `ac add` commands map to **UPSERT**-style writes for the same primary keys (same `id`): a second run against the same catalog typically **updates** the row (e.g. timestamps) rather than failing. If you ever see a duplicate-key failure on a given CLI version, treat that catalog as dirty and use a **fresh disposable** `DOLT_DB` or reset the disposable catalog before re-import.

This script does **not** delete rows first and does **not** run raw SQL cleanup — by design for reverse-spec (no production code, no manual delete harness required here).

## Canonical vs disposable

Under `with-isolated-test-profile`, rows land in the **disposable** `DOLT_DB` for that run. To curate the **canonical** project catalog, run the same script (or equivalent `spec add` / `ac add` sequence) against your normal dev catalog with env you intend to keep — only when you accept writing those rows there.
