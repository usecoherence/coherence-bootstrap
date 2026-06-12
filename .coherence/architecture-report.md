# Architecture Report — coherence-bootstrap

Generated: 2026-06-12
Purpose: Module map + data flow for human/LLM architectural discussion.

---

## 1. Workspace Overview

```
coherence-bootstrap        (Cargo.toml — workspace root, [[bin]] entry point)
  |
  +-- crates/coherence-code-quality     (standalone)
  +-- crates/coherence-core-db          (main crate)
  +-- crates/coherence-core-db-tui      (depends on core-db)
  +-- crates/coherence-test-world       (standalone test harness)
  |
  +-- scripts/             (30 shell scripts, dispatched via make tool)
  +-- tests/               (integration tests)
  +-- src/main.rs          (facade binary — routes to crates)
```

### Dependency graph

```
coherence-bootstrap (binary)
  ├── coherence-code-quality   (codescene-xray)
  ├── coherence-core-db        (all DB/spec/verify commands)
  └── coherence-core-db-tui    (TUI browser, depends on coherence-core-db)

coherence-core-db-tui (dev-dependency)
  └── coherence-test-world     (test scaffolding: temp Dolt, git, env guards)
```

---

## 2. Crate: `coherence-core-db` (main crate)

**Version:** 0.2.0
**Purpose:** Dolt-backed spec/AC/codeintel storage + verification runner.
**Binary:** `coherence-core-db` (invoked as `coherence-bootstrap <cmd>` through the facade).

### Module map

```
crates/coherence-core-db/src/
|
+-- main.rs              binary entry (delegates to cli::run)
+-- lib.rs               re-exports public API (codeintel_repo, spec_store, etc.)
|
+-- cli.rs               dispatches ~20 subcommands
+-- models.rs            domain structs + enums (Spec, AC, CodeLocation, ...)
|
+-- db.rs                ConnectionConfig, connect(), MySQL socket/TCP resolution
+-- project_manifest.rs  read/write .coherence/project.toml, catalog naming
+-- migrations.rs        Refinery-based schema migration runner
|
+-- spec_store.rs        CRUD: specs, acceptance_criteria, concerns, relations
+-- ac_code_link_store.rs   CRUD: codeintel_code_locations, codeintel_ac_links
+-- ac_verification_store.rs  CRUD: verification results (latest-per-link cache)
|
+-- ac_verify.rs         shell verification runner (sh -c via test_command)
+-- ac_test_layout.rs    AC -> Rust test file path mapping
+-- ac_materialize_codeintel_ids.rs  deterministic SHA-256 IDs for codeintel
|
+-- evidence_store.rs    ADR-0005: per-run evidence files (.coherence/runs/)
+-- test_world_guard.rs  ADR-0004: isolation guard for mutating writes
|
+-- commands/
    +-- mod.rs
    +-- cli_parse.rs, doctor.rs, migrate.rs
    +-- db_ping.rs, db_cmd.rs, db_truncate.rs
    +-- db_export_jsonl.rs, db_import_jsonl.rs, db_list_databases.rs
    +-- spec_cmd.rs, ac_cmd.rs, ac_tests_cmd.rs
    +-- verify_ac_cmd.rs, verify_spec_cmd.rs
    +-- evidence_sample_cmd.rs, project_cmd.rs
    +-- project_init_cmd.rs, project_reset_cmd.rs
    +-- tui_cmd.rs, m0_smoke.rs, m1_spec_smoke.rs
    +-- drop_isolated_test_db.rs

tests/ (inside coherence-core-db crate)
  cli_parse_characterize.rs, cli_smoke.rs
  spec_cmd_characterize.rs, ac_cmd_characterize.rs
  verify_ac_cmd_characterize.rs, verify_spec_cmd_characterize.rs
  evidence_sample_cli.rs, keep_test_world_logging.rs
```

### How a CLI command flows

```
1. main.rs                          receives args
2. src/main.rs (bootstrap facade)   routes: code-quality -> coherence-code-quality
                                        tui    -> coherence-core-db-tui
                                        other  -> coherence-core-db::cli::run()
3. cli.rs                           parses subcommand, dispatches to commands/<cmd>.rs
4. commands/<cmd>.rs                calls db::connect(), uses spec_store / ac_verify / etc.
5. spec_store / ac_verify / etc.    issues SQL via mysql crate, returns domain structs
6. CLI serialises to stdout (TSV | JSONL)
```

### Key domain types (`models.rs`)

```
Spec               { id, slug, title, description, level, status, created_at, updated_at }
AcceptanceCriterion { id, spec_id, slug, title, intent, review_mode, risk_level, concerns[] }
SpecRelation       { spec_id_a, spec_id_b, relation_kind }
CodeLocation       { id, file_path, kind, ... }
AcCodeLink         { ac_id, code_location_id, relation_kind, test_command, ... }
SpecGraph          { specs: Vec<Spec>, relations: Vec<SpecRelation> }

AcVerifyAcRunResult    { ac_id, ... per-link statuses, overall }
VerifySpecRunResult    { spec_id, results: Vec<AcVerifyAcRunResult> }
```

Enums: `SpecLevel { Foundation | System | Product | Module | Component }`
       `SpecStatus { Draft | Active | Deprecated | Retired }`
       `ReviewMode { Manual | Pair | Team }`
       `RiskLevel { Low | Medium | High | Critical }`
       `ConcernKind`, `CodeLocationKind`, `AcCodeRelationKind`

---

## 3. Crate: `coherence-code-quality`

**Version:** 0.1.0
**Purpose:** CodeScene integration — local `cs review` + REST API metrics.

```
src/
  lib.rs             run(), run_xray()
  codescene_xray.rs  XrayReport, XrayFileMetrics, ApiConfig, --trigger
```

- No internal deps (only `serde_json`).
- Invoked as `coherence-bootstrap code-quality codescene-xray <file>`.
- `--trigger` posts to `/run-analysis`, polls until complete, fetches metrics.

---

## 4. Crate: `coherence-core-db-tui`

**Version:** 0.1.0
**Purpose:** ratatui-based terminal browser for spec/AC tree.

```
src/
  main.rs           binary entry
  lib.rs            run_terminal() — event loop
  app.rs            AppState, Screen enum (ProjectPicker | EnvPicker | Specs)
  action.rs         AppAction enum, key_to_action()
  update.rs         update() -> Vec<Effect>
  effects.rs        Effect enum, execute_effects()
  ui.rs             rendering
  tree.rs           TreeItem, build_tree(), toggle_expand()
  edit.rs           Draft (spec/ac editing with dirty tracking)
  repository.rs     SpecRepository trait, DoltSpecRepository impl
  theme.rs          color constants
  project_discovery.rs  discover ~/git/ for .coherence/project.toml
```

- Three screens: ProjectPicker -> EnvPicker -> Specs (hierarchical tree).
- Verification status markers: `[+]` pass, `[!]` fail, `[?]` pending, `[-]` no links, `[ ]` not run.
- Editing via `$EDITOR` for description/intent, keyboard cycling for enums.

---

## 5. Crate: `coherence-test-world`

**Version:** 0.1.0
**Purpose:** Reusable test scaffolding — temp dirs, Dolt servers, env guards.

```
src/
  world.rs      World enum (Filesystem | Command | Dolt), VerificationResult
  scaffold.rs   Scaffold (temp dir with helpers)
  dolt_world.rs DoltWorld, DoltServer (temp Dolt data dir, start server)
  recipe.rs     E2eRecipe builder (Scaffold + DoltWorld + migrations + seed)
  service.rs    Service trait, Services collection
  env_guard.rs  EnvGuard (save/restore env + cwd)
```

- Used by `ac_project-env-selection.rs` integration test.
- Command boundary: AC checks run as `sh -c` subprocesses (language-agnostic).

---

## 6. `src/main.rs` — Bootstrap Facade

```
coherence-bootstrap <cmd>
  |
  +-- code-quality codescene-xray    -> coherence-code-quality
  +-- tui                            -> coherence-core-db-tui
  +-- version | --version | -V       -> print version
  +-- help | --help | -h             -> print help
  +-- (everything else)              -> coherence-core-db::cli::run()
```

Catch-all routes to `coherence-core-db` which handles:
`spec`, `ac`, `ac-tests`, `db`, `db-ping`, `db-list-databases`,
`drop-isolated-test-db`, `verify-ac`, `verify-spec`, `evidence-sample`,
`project`, `doctor`, `migrate`, `m0-smoke`, `m1-spec-smoke`, `help`.

---

## 7. Scripts Layer (`scripts/`)

30 scripts, dispatched via `make tool <cmd>` → `scripts/tool <cmd>` → `scripts/<cmd>`.

```
make tool bootstrap     -> scripts/bootstrap        (first-time: git init + project init + migrate)
make tool doctor        -> scripts/doctor            (dependency + repo state check)
make tool context       -> scripts/context           (product statement + current state)
make tool next          -> scripts/next              (beads task fetch)
make tool run           -> scripts/run               (fmt + clippy + test under isolation)
make tool present-work  -> scripts/present-work       (session summary)
make tool feedback      -> scripts/feedback           (next-steps guidance)

Dolt lifecycle:
  scripts/dolt-start       start sql-server (repo-local or user-scoped)
  scripts/dolt-stop        stop sql-server
  scripts/dolt-status      print connection state
  scripts/dolt-layout.sh   sourced library: path/mode helpers

Testing:
  scripts/with-isolated-test-profile   wrap cmd in disposable Dolt DB
  scripts/test-world-reset             drop .dolt/ (repo-local mode)

Code quality:
  scripts/codescene-env, codescene-install, codescene-delta, codescene-full

Demo:
  scripts/demo-container-*

Policy:
  scripts/check-policy       no root markdown, no manual docs edits
  scripts/reverse-spec-inventory  Python: SCIP-based code inventory
```

---

## 8. Dolt DB Architecture

### Four surfaces (from AGENTS.md)

| Surface | What | Store |
|---------|------|-------|
| Canonical catalog | Curated spec/AC/codeintel rows | Dolt database (`slug_hash_env`) |
| Isolated test world | Disposable per-test DB | `coherence_test_<uuid>` |
| Per-run evidence | Verification snapshots | `.coherence/runs/<run-id>/` files |
| User-scoped service | Shared dolt sql-server | `~/.local/share/coherence/db/` data dir |

### Connection resolution (`db.rs` / `ConnectionConfig::from_env()`)

```
DOLT_DB is set?        YES -> use it (override, skip manifest)
                       NO  -> read .coherence/project.toml

  project_hash exists? YES -> normalized name: "{slug}_{hash}_{env}"
                       NO  -> legacy dolt_db_name
                       NO  -> fallback: cwd basename / "dolt"

COHERENCE_ENV: dev (default), test, prod  -> suffix _dev, _test, _prod
```

### SQL schema modules

```
sql/modules/
  spec/
    migrations/
      V1__initial.sql, V2__cascade_relations.sql, V3__default_level.sql, ...
  codeintel/
    migrations/
      V1__initial.sql, V2__ac_verification.sql, ...
```

Two independent Refinery history tables: `refinery_schema_history` (spec) and `refinery_schema_history_codeintel` (codeintel). Both live in the same physical DB.

---

## 9. Agent Dolt Connectivity Guide

**Problem:** Raw `dolt sql` CLI does NOT reliably show the same databases as the Rust CLI.
The `dolt sql` command starts its own engine (local `.dolt/` dir), NOT the running server,
even when `DOLT_SOCKET` is set.

**Correct ways to query Dolt state:**

```bash
# 1. Preferred — use the coherence-core-db CLI
cargo run -p coherence-core-db --bin coherence-core-db -- spec list
cargo run -p coherence-core-db --bin coherence-core-db -- ac list --spec-id <SPEC_ID>
cargo run -p coherence-core-db --bin coherence-core-db -- db list-databases
cargo run -p coherence-core-db --bin coherence-core-db -- db ping

# 2. Via make tool
make tool dolt-status

# 3. Direct MySQL client connection (if mysql CLI is available)
#    socket path from scripts/dolt-layout.sh or make tool dolt-status
mysql -S /run/user/1000/coherence/dolt.sock -e "SHOW DATABASES;"

# 4. dolt sql-server query (set env properly first)
DOLT_SOCKET=/run/user/1000/coherence/dolt.sock \
  dolt sql-client -q "SELECT * FROM coherence_core_db_bootstrap_2a2c_dev.specs;"

# WRONG — this starts a local engine, not the server:
#   dolt sql -q "SELECT ..."
```

The Rust CLI always connects to the running `dolt sql-server` via MySQL protocol
over Unix socket (with optional TCP fallback). Use it for authoritative queries.

---

## 10. Spec/AC Sync Status (from commit 83c2d9ee)

**Commit `83c2d9ee`** added `coherence-code-quality` crate:
- `crates/coherence-code-quality/` (new crate: codescene-xray)
- `.coherence/exports/bootstrap-specs.jsonl` updated: +1 spec + 6 ACs

Bootstrap-specs.jsonl sync with Dolt DB:
- **Specs:** 41 in JSONL, 41 in Dolt DB ✅
- **ACs:** 244 in JSONL, verified in Dolt DB for codescene-xray spec ✅
- Full AC count in Dolt needs per-spec `ac list` queries (no bulk AC dump command yet)

**Conclusion:** JSONL export ↔ Dolt DB are in sync for the codescene-xray addition.
No missing specs or ACs detected.
