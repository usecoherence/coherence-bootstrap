# PR-3.5 — Link L0 world harness into the catalog (`verified_by`)

**Goal:** `verify-ac AC-CLI-SURFACE-HELP` (and VERSION / UNKNOWN) runs the existing world runner:

```text
.coherence/reverse-spec/world/bin/run-ac <AC_ID>
```

**Scope:** SQL upsert into `codeintel_code_locations` + `codeintel_ac_links` only. No `crates/**` changes, no new CLI behavior.

**Depends on:** PR-2 rows (`CLI-COMMAND-SURFACE-SPEC` + AC ids) already in the **same** logical catalog you pass to `verify-ac`.

## How to run

From repository root, with the **same** env as `coherence-core-db spec list` / `verify-ac` (see `../pr2-catalog-import/README.md`):

```bash
export COHERENCE_CORE_DB_BIN="$PWD/target/debug/coherence-core-db"
./scripts/with-isolated-test-profile bash -c '
  ./.coherence/reverse-spec/pr2-catalog-import/import-cli-command-surface.sh
  ./.coherence/reverse-spec/pr3-verified-by-links/import-l0-world-links.sh
  "$COHERENCE_CORE_DB_BIN" verify-ac AC-CLI-SURFACE-HELP
  "$COHERENCE_CORE_DB_BIN" verify-ac AC-CLI-SURFACE-VERSION
  "$COHERENCE_CORE_DB_BIN" verify-ac AC-CLI-SURFACE-UNKNOWN
'
```

Or import only (then verify in separate shells, still from repo root):

```bash
./scripts/with-isolated-test-profile bash -c '
  ./.coherence/reverse-spec/pr2-catalog-import/import-cli-command-surface.sh
  ./.coherence/reverse-spec/pr3-verified-by-links/import-l0-world-links.sh
'
```

Then (still from repo root so relative `test_command` resolves):

```bash
./scripts/with-isolated-test-profile coherence-core-db verify-ac AC-CLI-SURFACE-HELP
./scripts/with-isolated-test-profile coherence-core-db verify-ac AC-CLI-SURFACE-VERSION
./scripts/with-isolated-test-profile coherence-core-db verify-ac AC-CLI-SURFACE-UNKNOWN
```

Or with explicit binary:

```bash
export COHERENCE_CORE_DB_BIN="$PWD/target/debug/coherence-core-db"
./scripts/with-isolated-test-profile "$COHERENCE_CORE_DB_BIN" verify-ac AC-CLI-SURFACE-HELP
```

## Catalog name resolution

`DOLT_DB` wins when set. Otherwise the script runs `coherence-core-db doctor` to a **temp file** (not a pipe) and parses `effective_catalog_without_DOLT_DB_override` — avoids Rust **Broken pipe** panics when `doctor` stdout is closed early by `awk`.

## SQL transport

The import script executes SQL via:

1. **`mysql` / `mariadb`** when available and **`DOLT_SOCKET`** is set (typical mysql client over Unix socket).
2. Else **`dolt --no-tls --host … --port … sql`** using **`DOLT_PORT`** or **`.coherence/run/dolt.tcp_port`**, then **`3306`** (matches repo-local `scripts/dolt-start` defaults).

Override: **`COHERENCE_REVERSE_SPEC_SQL`** — if set to an executable, it is invoked as  
`"$COHERENCE_REVERSE_SPEC_SQL" <sql-file>`  
(stdin is the SQL batch); exit non-zero fails the import.

## Evidence

Each successful `run-ac` writes under `world/evidence/<run-id>/<AC_ID>/` (see `../world/README.md`). After `verify-ac`, inspect that tree if a check fails.
