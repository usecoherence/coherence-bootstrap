# Technical: world dependencies (aggregate)

**Classification:** observed + pointers to AGENTS — what the CLI **typically** needs, not exhaustive formalism.

## Connection / catalog

- **`ConnectionConfig::from_env()`** — resolves `DOLT_SOCKET`, `DOLT_DB`, manifest under git root, `COHERENCE_ENV`, optional user-scoped Dolt layout, etc. (see `db.rs`, AGENTS).
- **Manifest preflight** — `migrate` and `db-ping` call `manifest_catalog_preflight_for_connect` before connecting (fail fast with remediation text).
- **Migrations** — many paths call `migrations::apply_all` before CRUD or verification so schema matches embedded Refinery sets (spec + codeintel histories).

## Process / filesystem

- **Current working directory** — manifest discovery, workspace resolution for `ac-tests` / `evidence-sample` (walk up for `AGENTS.md` or `--workspace`).
- **`.coherence/project.toml`** — project identity, catalog name derivation when `DOLT_DB` unset.
- **`.coherence/runs/`** — evidence-sample and optional verify-ac evidence hooks.
- **`tests/ac/**`** — ac-tests materialize/check targets under chosen workspace.

## Isolation / policy

- **`COHERENCE_DB_PROFILE=test`** — required for mutating smoke (`m0-smoke`, `m1-spec-smoke`) and guarded test writes; see `test_world_guard`.
- **`drop-isolated-test-db`** — meaningful mainly under user-scoped Dolt + disposable DB prefix.

## External processes

- **`verify-ac` / `verify-spec`** — spawn `sh -c` for each runnable `verified_by` link with non-empty `test_command`.
- **Dolt / MySQL protocol** — server must accept connections on configured socket or TCP path used by `mysql` crate.

## Reverse-spec note (world harness, future)

For black-box verification later: treat the above as **capabilities the harness must provision** (container/Dagger/temp git + Dolt + env), not as reasons to edit v0 source.
