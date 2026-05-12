# Reverse-spec draft (v0 observed behavior)

**Purpose:** scratch Markdown aligned with **current Dolt schema column names** (`specs`, `acceptance_criteria`, `acceptance_criterion_concerns`, `codeintel_code_locations`, `codeintel_ac_links`). Intended for review and later import via `spec add` / `ac add` (or a seed pipeline), not as canonical catalog truth until loaded.

**Reverse-spec agent constraint (v0 oracle):** do **not** change production sources (`crates/coherence-core-db/src/**`, crate `Cargo.toml`, migrations, CLI behavior) to “make reverse-spec easier”. Treat the built binary as a **black box**; seed the catalog via **CLI / Dolt SQL / seed files**; automated checks should spawn `coherence-core-db` via `std::process::Command` only. If the system cannot be exercised without code changes, record that as a **finding / future refactor**, not a silent fix in the same pass.

**CLI inventory (markdown-only, read-only code):** command surface, command groups, and one-layer technical maps — **`.coherence/reverse-spec/cli/`** (see `cli/README.md`). Use that tree for “what the CLI product is”; the Phase-1 `verify-ac` bridge below is a **later** catalog/harness step, not the first inventory milestone.

**Source:** `source: observed legacy behavior from coherence-core-db-bootstrap` (executable v0 specimen).

**Classification (do not mix in one AC):**

- **observed** — what the binary does today; reverse-spec source of truth for this pass.
- **desired** — future architecture; not asserted here (e.g. `verify-spec --strict` failing when an AC has no `verified_by` — **not** in this draft).
- **compatibility** — explicitly call out when an AC is meant to survive refactors.
- **deprecated / accidental** — recorded as observed only; not a long-lived contract.
- **observed / accidental candidate** — v0 behavior we record to avoid surprise, but **do not** promote to compatibility by default; may change under stricter policy modes later.

**First catalog-verification milestone (explicit non-goal: “complete reverse engineering”):** close the **smallest** closed loop — **not** “import three SPECs at once”, and **not** `verify-ac` + `verify-spec` in the same first harness change-set (`verify-spec` aggregation already wants a second AC or a composed failure case — that belongs in the next step).

**Phase 1 harness — slice 1 — prove only the `verify-ac` bridge (emotional center of Phase 1):**

1. Import **only** SPEC `coredb-ac-verification-loop` and **only** AC `legacy-v0-ac-cdb-ver-001` (`verified-by-link-runnable`).
2. Insert one `codeintel_code_locations` row: `test_command = "true"` (shell via existing runner — **no** `cargo test`, **no** `tests/ac/**` materialization; that is Phase 4).
3. Insert one `codeintel_ac_links` row: `relation_kind = verified_by`.
4. Run `verify-ac <AC_ID>` → assert exit **0**.
5. **Negative:** update the same location’s `test_command` to `"false"` (or an equivalent fixture row) → `verify-ac <AC_ID>` → assert exit **non-zero**.

That proves the core bridge only: **AC → codeintel `verified_by` → executable command → `verify-ac` exit code**.

**Phase 1 harness — slice 2 — prove `verify-spec` aggregation (separate small change-set):**

- Add AC `legacy-v0-ac-cdb-ver-002` and the graph rows needed for **two** ACs under the same spec / one forced failure → `verify-spec <SPEC_ID>` exits non-zero when any executed link fails.

**Phase 1 harness — slice 3 or later —** AC `legacy-v0-ac-cdb-ver-003` (`no-verified-by-exit-zero`): **not** part of the first green build; convenience / accidental-candidate semantics should not anchor the first win.

**After harness slice 1 (+ slice 2 when ready):** use the same bridge for catalog CRUD, migrate/bootstrap, then materialization (Phases 2–4).

**Execution order (reverse-spec backlog — do not start at SPEC “1” in document order):**

| Phase | SPEC slug | Role |
|-------|-----------|------|
| **1** | `coredb-ac-verification-loop` | **Start here.** Heart of Coherence: AC → codeintel `verified_by` → executable command → **`verify-ac` first (harness slice 1), then `verify-spec` (slice 2)**. Proves the spec graph is semantically tied to behavior. |
| **2** | `coredb-spec-ac-catalog` | Catalog semantics: spec/AC round-trip, slug uniqueness — verified **using** the Phase 1 bridge. |
| **3** | `coredb-catalog-bootstrap` | Infra: migrate, `db-ping`, manifest / binding preflight — same. |
| **4** | `coredb-ac-test-materialization` | Graph → filesystem test artifacts (`ac-tests materialize-rust`), non-overwrite, linkability — **not** pure catalog CRUD; separate layer from Phase 2. |

The sections below follow **phase order** (verification loop first). Numeric labels in headings are phase ids, not priority of “SPEC 1” from an earlier draft.

**Verification hooks by phase:**

| Phase | Primary commands / entrypoints |
|-------|--------------------------------|
| 1 | **Harness slice 1:** `verify-ac` only + `codeintel_*` + minimal SPEC/AC rows. **Slice 2:** `verify-spec`. (Do not require `verify-spec` in slice 1.) |
| 2 | `spec add\|list\|show`, `ac add\|list\|show` |
| 3 | `project …`, `migrate`, `db-ping`, optionally `doctor` |
| 4 | `ac-tests materialize-rust` (+ optional `check-rust` when in scope) |

**Timestamps:** replace `PLACEHOLDER_ISO8601` on import with real `created_at` / `updated_at` if the loader does not set them automatically.

---

## Phase 1 — SPEC `coredb-ac-verification-loop`

**Claim:** An AC can be linked to an executable verification command via codeintel; `verify-ac` and `verify-spec` execute linked commands and aggregate results. **This is the recommended first vertical slice:** the legacy oracle first proves it can verify *itself* through `verified_by`, then everything else is reverse-spec’d through that machine.

**Backlog vs implementation:** this SPEC lists **three** AC rows below (`ver-001` … `ver-003`) for completeness of the reverse-spec backlog. **Implement in thin harness slices:** slice 1 = only `ver-001` + shell `true`/`false`; slice 2 = `ver-002`; defer `ver-003` so “no link exits 0” does not become the emotional center of the first green build.

### Row: `specs`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-spec-coredb-ac-verification-loop` |
| `slug` | `coredb-ac-verification-loop` |
| `title` | AC verification via codeintel and verify-ac/spec |
| `description` | **Classification: observed.** `codeintel_code_locations` holds `test_command` for runnable locations; `codeintel_ac_links` with `relation_kind = verified_by` connects ACs to those locations; `verify-ac` / `verify-spec` consult the graph and exit according to command success. |
| `level` | `module` |
| `status` | `draft` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

### Rows: `acceptance_criteria`

#### AC `legacy-v0-ac-cdb-ver-001`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-ver-001` |
| `spec_id` | `legacy-v0-spec-coredb-ac-verification-loop` |
| `slug` | `verified-by-link-runnable` |
| `title` | verified_by link runs shell command |
| `intent` | **Classification: observed.** Given an AC and a `codeintel_code_locations` row with non-empty `test_command`, and a `codeintel_ac_links` row (`relation_kind` = `verified_by`) tying the AC to that location, `verify-ac <AC_ID>` executes the command via the shell runner and exits non-zero on command failure. |
| `review_mode` | `automated` |
| `risk_level` | `high` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-ver-001`, `correctness`).

**Verification path (planned):** end-to-end `verify-ac` with shell **`true`** (exit 0) then **`false`** (exit non-zero). **Do not** use `cargo test` or `tests/ac/**` here — that couples Phase 1 to Phase 4 materialization. The runner only accepts code location `kind` values that map to **test_file** or **test_command** in the implementation; for pure shell, use **`test_command`** as the DB `kind` string (see `CodeLocationKind` in code).

**Planned rows (fill ids on import) — harness slice 1 minimal:**

`codeintel_code_locations`

| Column | Example value |
|--------|----------------|
| `id` | `PLACEHOLDER_LOC_VERIFY_001` |
| `repo_path` | `.` |
| `file_path` | `.coherence/reverse-spec/phase1-shell-bridge` (placeholder path; not read for `test_command` execution — avoids pulling materialized Rust tests into harness slice 1) |
| `kind` | `test_command` |
| `symbol` | `NULL` or empty |
| `test_command` | `true` (then negative case: update to `false`) |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

`codeintel_ac_links`

| Column | Example value |
|--------|----------------|
| `id` | `PLACEHOLDER_LINK_VERIFY_001` |
| `ac_id` | `legacy-v0-ac-cdb-ver-001` |
| `code_location_id` | `PLACEHOLDER_LOC_VERIFY_001` |
| `relation_kind` | `verified_by` |
| `note` | reverse-spec v0 bridge |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

---

#### AC `legacy-v0-ac-cdb-ver-002` (**Harness slice 2 — not bundled with slice 1**)

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-ver-002` |
| `spec_id` | `legacy-v0-spec-coredb-ac-verification-loop` |
| `slug` | `verify-spec-aggregates` |
| `title` | verify-spec aggregates AC verification |
| `intent` | **Classification: observed.** `verify-spec <SPEC_ID>` runs verification for each eligible AC under that spec and fails if any constituent `verify-ac` would fail (any executed `verified_by` link failed). |
| `review_mode` | `automated` |
| `risk_level` | `high` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-ver-002`, `correctness`).

**Verification path (planned):** two ACs under one spec, one forced failure, assert non-zero `verify-spec`.

---

#### AC `legacy-v0-ac-cdb-ver-003` (**Harness slice 3+ — not first green build**)

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-ver-003` |
| `spec_id` | `legacy-v0-spec-coredb-ac-verification-loop` |
| `slug` | `no-verified-by-exit-zero` |
| `title` | AC without verified_by exits zero |
| `intent` | **Classification: observed / accidental candidate.** An AC with no `verified_by` rows yields `no_verification_links`; `verify-ac` exits **0** and does not spawn a shell for that AC. `verify-spec` counts such ACs in `no_verification` and still exits **0** if no executed link failed. This is **convenience behavior in v0**, not promoted to **compatibility**: a future `verify-spec --strict` (or policy mode) may require every AC to have verification. User-visible `SUMMARY` / tab-separated lines are **not** part of the contract unless separately promoted to UX-spec. |
| `review_mode` | `automated` |
| `risk_level` | `low` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-ver-003`, `correctness`).

**Verification path (planned):** mixed spec: one AC with link, one without; assert aggregate exit code matches **current** implementation (zero when no failures).

---

## Phase 2 — SPEC `coredb-spec-ac-catalog`

**Claim:** The system can create, list, and show specs and acceptance criteria with stable identity and per-spec AC slug semantics. **Depends on Phase 1** for proving behaviors via `verify-ac` / `verify-spec` once links exist.

### Row: `specs`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-spec-coredb-spec-ac-catalog` |
| `slug` | `coredb-spec-ac-catalog` |
| `title` | Spec and acceptance-criterion catalog CRUD |
| `description` | **Classification: observed.** Operators can persist specs and ACs via CLI (`spec`, `ac`), read them back by id, and rely on unique `(spec_id, slug)` for ACs. Materialization of test files from the graph is **not** in this SPEC — see Phase 4. |
| `level` | `module` |
| `status` | `draft` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

### Rows: `acceptance_criteria`

#### AC `legacy-v0-ac-cdb-sac-001`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-sac-001` |
| `spec_id` | `legacy-v0-spec-coredb-spec-ac-catalog` |
| `slug` | `spec-round-trip` |
| `title` | Spec add/list/show round-trip |
| `intent` | **Classification: observed.** Given a migrated catalog, `spec add` persists a spec; `spec list` includes it; `spec show` returns the same stable `id`, `slug`, `title`, `description`, `level`, and `status` fields as stored in `specs`. |
| `review_mode` | `automated` |
| `risk_level` | `high` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-sac-001`, `correctness`).

**Verification path (planned):** CLI integration or `m1-spec-smoke`-class exercise.

---

#### AC `legacy-v0-ac-cdb-sac-002`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-sac-002` |
| `spec_id` | `legacy-v0-spec-coredb-spec-ac-catalog` |
| `slug` | `ac-round-trip` |
| `title` | AC add/list/show round-trip |
| `intent` | **Classification: observed.** Given an existing spec, `ac add` persists an AC with non-empty `intent`; `ac list` and `ac show` surface stable `id`, `spec_id`, `slug`, `title`, `intent`, `review_mode`, and `risk_level` consistent with `acceptance_criteria` rows. |
| `review_mode` | `automated` |
| `risk_level` | `high` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-sac-002`, `correctness`).

**Verification path (planned):** CLI integration.

---

#### AC `legacy-v0-ac-cdb-sac-003`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-sac-003` |
| `spec_id` | `legacy-v0-spec-coredb-spec-ac-catalog` |
| `slug` | `ac-slug-uniqueness-per-spec` |
| `title` | AC slug unique within spec |
| `intent` | **Classification: observed / compatibility.** Two AC rows for the same `spec_id` cannot share the same `slug` (DB unique index `idx_acceptance_criteria_spec_id_slug`); attempts to violate this fail predictably (error or refusal), preserving stable addressing for verification and materialization. |
| `review_mode` | `automated` |
| `risk_level` | `medium` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-sac-003`, `correctness`).

**Verification path (planned):** negative test via CLI or store API.

---

## Phase 3 — SPEC `coredb-catalog-bootstrap`

**Claim:** A repository-local Coherence catalog can be initialized (manifest + bind where applicable) and reached over the MySQL protocol (Dolt), including applied module migrations. **Infra / “ordinary” slice** — valuable, but **after** the verification machine (Phase 1) exists.

### Row: `specs`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-spec-coredb-catalog-bootstrap` |
| `slug` | `coredb-catalog-bootstrap` |
| `title` | Repository-local Coherence DB bootstrap |
| `description` | **Classification: observed.** After operator bootstrap (manifest, optional `project init` bind, Dolt reachable, `migrate`), the process can answer `db-ping` successfully against the configured socket/TCP target and the spec module tables exist per migrations. Exact CLI sequence may vary; behavioral outcome is a reachable migrated catalog. |
| `level` | `system` |
| `status` | `draft` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

### Rows: `acceptance_criteria`

#### AC `legacy-v0-ac-cdb-cat-001`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-cat-001` |
| `spec_id` | `legacy-v0-spec-coredb-catalog-bootstrap` |
| `slug` | `manifest-and-binding-preflight` |
| `title` | Project manifest supports catalog resolution |
| `intent` | **Classification: observed.** Given a git work tree with `.coherence/project.toml` containing a non-empty `project_slug`, and either a bound `project_hash` (or legacy `dolt_db_name`) or operator-set `DOLT_DB`, catalog preflight paths used by connect/migrate accept the configuration (no spurious refusal solely due to missing slug). |
| `review_mode` | `automated` |
| `risk_level` | `high` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-cat-001`, `correctness`).

**Verification path (planned):** integration or CLI against `project` / `doctor` / `migrate` preflight — assert exit success and resolved catalog identity, not exact stderr wording.

---

#### AC `legacy-v0-ac-cdb-cat-002`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-cat-002` |
| `spec_id` | `legacy-v0-spec-coredb-catalog-bootstrap` |
| `slug` | `migrate-applies-spec-module` |
| `title` | Migrate creates spec-module tables |
| `intent` | **Classification: observed.** After `migrate` against the target catalog, tables defined by the spec module migrations exist (at minimum `specs`, `acceptance_criteria`, `acceptance_criterion_concerns`, `spec_relations` as created by embedded SQL). |
| `review_mode` | `automated` |
| `risk_level` | `high` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-cat-002`, `correctness`).

**Verification path (planned):** SQL introspection or store smoke (`m1-spec-smoke` / direct query) after migrate.

---

#### AC `legacy-v0-ac-cdb-cat-003`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-cat-003` |
| `spec_id` | `legacy-v0-spec-coredb-catalog-bootstrap` |
| `slug` | `db-ping-readiness` |
| `title` | db-ping reports MySQL-protocol readiness |
| `intent` | **Classification: observed.** Given a running Dolt sql-server compatible target and env pointing `DOLT_SOCKET` / `DOLT_HOST`+`DOLT_PORT` as implemented, `db-ping` exits successfully when the server accepts connections. |
| `review_mode` | `automated` |
| `risk_level` | `medium` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-cat-003`, `reliability`).

**Verification path (planned):** CLI `db-ping` in isolated or local fixture.

---

#### AC `legacy-v0-ac-cdb-cat-004`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-cat-004` |
| `spec_id` | `legacy-v0-spec-coredb-catalog-bootstrap` |
| `slug` | `migrate-applies-codeintel-module` |
| `title` | Migrate creates codeintel tables |
| `intent` | **Classification: observed.** After `migrate`, codeintel module tables exist (`codeintel_code_locations`, `codeintel_ac_links`) per embedded migrations. |
| `review_mode` | `automated` |
| `risk_level` | `medium` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-cat-004`, `correctness`).

**Verification path (planned):** SQL introspection or migrate integration test.

---

## Phase 4 — SPEC `coredb-ac-test-materialization`

**Claim:** The graph can drive creation of Rust test artifacts on disk (`ac-tests materialize-rust`) without corrupting existing files; generated paths/commands can be linked back through codeintel for `verified_by`. **Semantically distinct from catalog CRUD** (Phase 2): this is graph → filesystem → verification bridge.

### Row: `specs`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-spec-coredb-ac-test-materialization` |
| `slug` | `coredb-ac-test-materialization` |
| `title` | AC test materialization from catalog graph |
| `description` | **Classification: observed.** `ac-tests materialize-rust` reflects live `specs` / `acceptance_criteria` (and related conventions) into missing files under `tests/ac/**`; linkage to `verified_by` commands is the handoff to Phase 1. |
| `level` | `module` |
| `status` | `draft` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

### Rows: `acceptance_criteria`

#### AC `legacy-v0-ac-cdb-mat-001`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-mat-001` |
| `spec_id` | `legacy-v0-spec-coredb-ac-test-materialization` |
| `slug` | `materialize-rust-creates-missing-tests` |
| `title` | materialize-rust creates missing AC test files |
| `intent` | **Classification: observed.** `ac-tests materialize-rust` creates missing files under `tests/ac/**` derived from the live graph. |
| `review_mode` | `automated` |
| `risk_level` | `medium` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-mat-001`, `maintainability`).

**Verification path (planned):** CLI `ac-tests materialize-rust` + filesystem assertions.

---

#### AC `legacy-v0-ac-cdb-mat-002`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-mat-002` |
| `spec_id` | `legacy-v0-spec-coredb-ac-test-materialization` |
| `slug` | `materialize-rust-no-overwrite` |
| `title` | materialize-rust does not overwrite existing files |
| `intent` | **Classification: observed.** Existing files under `tests/ac/**` are never overwritten by `materialize-rust`; reruns are idempotent at the filesystem level for already-present targets. |
| `review_mode` | `automated` |
| `risk_level` | `high` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-mat-002`, `reliability`).

**Verification path (planned):** pre-create file, run materialize, assert content unchanged.

---

#### AC `legacy-v0-ac-cdb-mat-003`

| Column | Value |
|--------|--------|
| `id` | `legacy-v0-ac-cdb-mat-003` |
| `spec_id` | `legacy-v0-spec-coredb-ac-test-materialization` |
| `slug` | `materialized-artifact-linkable` |
| `title` | Generated test command can be tied via codeintel |
| `intent` | **Classification: observed.** After materialization, the repository convention allows (or tooling upserts) `codeintel_code_locations` + `codeintel_ac_links` so the new test path participates in `verified_by` verification (handoff to Phase 1). Exact mechanism (manual CLI vs upsert) is implementation detail; outcome is linkability. |
| `review_mode` | `automated` |
| `risk_level` | `medium` |
| `created_at` | `PLACEHOLDER_ISO8601` |
| `updated_at` | `PLACEHOLDER_ISO8601` |

**`acceptance_criterion_concerns`:** (`legacy-v0-ac-cdb-mat-003`, `correctness`).

**Verification path (planned):** materialize → assert graph rows or run `verify-ac` for the AC once linked.

---

## Appendix — CLI surface (inventory only; not full spec coverage)

From `cli.rs` top-level commands: `help`, `doctor`, `migrate`, `db-ping`, `drop-isolated-test-db`, `m0-smoke`, `m1-spec-smoke`, `spec`, `ac`, `ac-tests`, `verify-ac`, `verify-spec`, `evidence-sample`, `project`, `version`.

Subcommands for `spec` / `ac` / `ac-tests` / `project` are implemented in respective command modules — extend this draft when those paths become part of a chosen vertical slice.

---

## `spec_relations` (optional, not used in v0 draft)

No rows in this draft. When decomposing SPECs, use `spec_relations` with columns `id`, `source_spec_id`, `target_spec_id`, `relation_kind`, `note`, `created_at`, `updated_at` (same names as migration `V1__create_spec_tables.sql`).
