# Commands: DB / runtime — `migrate`, `db-ping`, `doctor`, `project`, `drop-isolated-test-db`

**Classification:** observed.

## `migrate`

**Public intent:** Apply embedded Refinery migrations (spec module then codeintel module orchestration).

**World:** manifest catalog preflight when `DOLT_DB` not overriding; then `ConnectionConfig::from_env`, `migrations::apply_all`.

**Observable:** stdout `migrate: success`, database name, applied migration count; stderr + **1** on failure.

---

## `db-ping`

**Public intent:** Prove MySQL-protocol reachability (socket-first behavior per `db` implementation).

**World:** same preflight as migrate; then `db::ping_server`.

**Observable:** stdout `db-ping: ok (…)` with mode; **1** on failure.

---

## `doctor`

**Public intent:** Operator sanity snapshot — toolchain hints, policy strings, manifest presence lines, effective catalog hints.

**World:** reads env and optionally manifest paths; **does not** open a DB connection for ping in this implementation path.

**Observable:** many `println!` lines to stdout; always **0** in current code path (`run`).

**Classification note:** “success” here means “printed diagnostics”, not “database healthy”.

---

## `project`

**Public intent:** Manifest / git binding lifecycle and lightweight preflight.

**Subcommands:**

| Sub | Role |
|-----|------|
| `catalog-preflight` | `manifest_catalog_preflight_for_connect`; stdout `ok` or stderr error (**1**). |
| `init` | Delegates `project_init_cmd` (bind hash, manifest writes — see AGENTS). |
| `reset` | No extra args; delegates `project_reset_cmd` (repair bind + migrate). |

**Usage errors:** missing subcommand prints multi-line usage to stderr, exit **64**; unknown subcommand **64**; `reset` with extra args **64**.

---

## `drop-isolated-test-db`

**Public intent:** Drop disposable `coherence_test_*` (prefix configurable) database on **user-scoped** Dolt when enabled.

**World:** if `COHERENCE_USE_USER_SCOPED_DOLT` not set → prints skip message, exit **0**. Otherwise connect and `DROP DATABASE` with prefix guard; refusal exit **2** if name not disposable.

**Classification:** operator / hygiene command tied to ADR-0004/0006 test worlds.
