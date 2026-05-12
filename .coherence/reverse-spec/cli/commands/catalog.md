# Commands: catalog — `spec`, `ac`

**Classification:** observed — `src/commands/spec_cmd.rs`, `ac_cmd.rs`.

## `spec`

**Public intent:** Create/read-style catalog management of rows in `specs` (M1 spec module): add, list, show — no update/delete subcommands in the current CLI.

**Subcommands:** `add` | `list` | `show` (unknown subcommand → error string, exit via `run` → **1**).

**Typical world:** migrated catalog reachable (`connect_migrated`: `ConnectionConfig::from_env`, `migrations::apply_all`, `db::connect`).

**One-layer technical map:**

- Parse flags via `cli_parse::parse_args`.
- `add`: required `--slug`, `--title`; optional `--description`, `--level` (default `module`), `--status` (default `draft`), `--id` (else time-based `SPEC-GEN-…`); validates enum strings against `SpecLevel` / `SpecStatus`; builds `Spec`, `spec_store::put_spec`, stdout confirmation.
- `list` / `show`: query `spec_store`, print rows / single row.

**Observable effects:** DB rows in `specs`; stdout listings or confirmations.

**Failure modes:** missing required flags, unknown level/status, DB/connect errors (prefixed `spec:` on stderr).

---

## `ac`

**Public intent:** Create/read-style catalog management of `acceptance_criteria` (+ optional `acceptance_criterion_concerns` on add); add, list, show only.

**Subcommands:** `add` | `list` | `show`.

**Typical world:** same `connect_migrated` pattern; `add` requires existing `spec_id`.

**One-layer technical map:**

- `add`: `--spec-id`, `--title` required; `--intent`, `--review-mode`, `--risk-level`, `--slug` (default derived from id), repeatable `--concern`; validates concern/review/risk enums; `spec_store::put_acceptance_criterion` (+ concern rows).
- `list` / `show`: scoped by `--spec-id` where applicable.

**Observable effects:** DB rows in `acceptance_criteria` / concerns tables.

**Failure modes:** same pattern as `spec` (`ac:` prefix).

**Compatibility / semantics note:** unique `(spec_id, slug)` enforced at store/DB level — important for verification and materialization addressing.
