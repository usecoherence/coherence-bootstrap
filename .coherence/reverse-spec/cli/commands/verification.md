# Commands: verification — `verify-ac`, `verify-spec`, `ac-tests`

**Classification:** observed — `verify_ac_cmd.rs`, `verify_spec_cmd.rs`, `ac_tests_cmd.rs`, domain in `ac_verify.rs`, stores in `ac_code_link_store.rs` / `spec_store.rs`.

## `verify-ac`

**Public intent:** For one AC id, evaluate every **`verified_by`** codeintel link whose location kind is runnable (`test_file` or `test_command`) and `test_command` non-empty; run `sh -c '<command>'` (optional cwd from `repo_path` resolution).

**Invocation:** `verify-ac <AC_ID>` or `verify-ac --ac-id <AC_ID>`.

**Typical world:** migrated DB; graph rows in `codeintel_*` + existing AC row (no hard FK enforced at DB layer in M1).

**Observable outputs:** tab-oriented lines (`OVERALL`, `SUMMARY`, `LINK`, …) to stdout; exit **0** unless any executed link fails (then **1**). AC with no `verified_by` links → overall `no_verification`, exit **0** (convenience semantics — treat as **accidental candidate** for long-term contract; see main reverse-spec draft).

**Failure modes:** usage errors, connect/migrate errors, `verify_acceptance_criterion` / shell spawn errors → stderr `verify-ac: …`, exit **1**.

---

## `verify-spec`

**Public intent:** Load spec by id, list its ACs, run the same per-AC verification as `verify-ac`, aggregate counts, exit non-zero if any AC’s verification would be non-zero.

**Invocation:** `verify-spec <SPEC_ID>` or `--spec-id`.

**Typical world:** same as `verify-ac`; spec row must exist.

**Observable outputs:** `SPEC\t…` summary line + per-link lines for each AC.

---

## `ac-tests`

**Public intent:** Bridge between **live spec graph in DB** and **Rust files under `tests/ac/**`** — materialize missing skeletons; hard-check that expected files exist.

**Subcommands:** `materialize-rust` | `check-rust`.

**Typical world:** migrated DB + writable workspace tree; workspace = `--workspace` or nearest ancestor with `AGENTS.md` else cwd.

**One-layer technical map:**

- Load graph via `spec_store::load_spec_graph` (and related), compute expected paths from `ac_test_layout`.
- `materialize-rust`: create missing files only; may upsert codeintel rows for materialized tests (see module implementation — ties graph to filesystem + linkage).
- `check-rust`: fail (**1**) if any expected file missing.

**Failure modes:** bad workspace path, graph/load errors, internal path validation (`tests/ac/`, no `..`).

**Classification note:** overlaps **catalog** (reads specs/ACs) and **verification** (feeds `verify-ac` story); kept here as “verification prep / graph-to-tests”.
