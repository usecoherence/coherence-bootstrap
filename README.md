This is bootstrap for [Coherence](https://usecoherence.dev/).

[![CodeScene Average Code Health](https://codescene.io/projects/81147/status-badges/average-code-health)](https://codescene.io/projects/81147)
[![CodeScene Hotspot Code Health](https://codescene.io/projects/81147/status-badges/hotspot-code-health)](https://codescene.io/projects/81147)
[![CodeScene Missed Goals](https://codescene.io/projects/81147/status-badges/missed-goals)](https://codescene.io/projects/81147)
[![CodeScene System Mastery](https://codescene.io/projects/81147/status-badges/system-mastery)](https://codescene.io/projects/81147)

Coherence makes it easy to write / maintain claims which are also known as Acceptance Criteria or ACs.

The goal of this bootstrap is to provide minimal framework I can use to write the full system.

Yes, `coherence` builds itself 🤣
Because it is a semantic graph. And you can describe semantic graph with a semantic graph... 💥 hope your mind didn't blow up

But really it's very simple idea: You have graph nodes, and you have graph edges.
And the only thing semantic graph is doing is just EXISTING.

Engineers making this semantic graph entirely in their head, but this doesn't scale when LLM writes code.
LLM writes too much code to be honest, and I can't keep up with it.

So this is how I'm gonnna do it.

Reviewing 250 ACs is much easier than 10k LoC.

## Table of Contents

- [The grammar in 30 seconds](#the-grammar-in-30-seconds)
- [Try demo](#try-demo)
- [Is this Gherkin/BDD ?](#is-this-gherkinbdd-)
- [Install On macOS Without Docker](#install-on-macos-without-docker)
- [Load The Bootstrap Spec Catalog](#load-the-bootstrap-spec-catalog)
- [CodeScene CLI](#codescene-cli)
- [First Demo: From Requirement To Verified AC](#first-demo-from-requirement-to-verified-ac)
- [WIP Coherence Specs aka "Normalized Spec Tree Decisions v3"](#wip-coherence-specs-aka-normalized-spec-tree-decisions-v3)
  - [Principle](#principle)
  - [Level Distribution](#level-distribution)
  - [Spec Tree (v3)](#spec-tree-v3)
  - [Actions Summary](#actions-summary)

## The grammar in 30 seconds

Coherence has three primitives:

1. A **spec** describes a promise the system makes. An outcome.
2. An **acceptance criterion** makes that promise falsifiable. What needs to happen to achieve outcome?
3. **Evidence** connects the claim to executable verification. How exactly it happens in code?

```bash
coherence-bootstrap spec add \
  --slug product/payment-api \
  --title "Payment API" \
  --level system

coherence-bootstrap ac add \
  --spec-id SPEC-payment-api \
  --slug rejects-expired-cards \
  --title "Rejects expired cards" \
  --intent "When a payment uses an expired card, the API returns HTTP 400 with code INVALID_CARD" \
  --review-mode automated \
  --risk-level high

coherence-bootstrap ac-tests materialize-rust \
  --ac-id AC-rejects-expired-cards
```

Coherence creates a durable, queryable relationship:

```text
SPEC: Payment API
  └── has
      AC: Rejects expired cards
        └── verified_by
            TEST: cargo test rejects_expired_cards
              └── reports
                  PASS / FAIL
```

The test remains ordinary code in your repository. Coherence does not replace your language, test framework, editor, or CI system. Use any language, as long as you can call the test programmatically.

It records **what the system promises, where that promise is implemented, and what evidence currently supports it**.

Instead of reviewing 10,000 lines of generated code, review the behavioral claims, and inspect the code only where the evidence or intent is uncertain.

## Try demo

```bash
git clone https://github.com/usecoherence/coherence-bootstrap
cd coherence-bootstrap
make demo-container-shell   # you need docker tho
```

Then ask the agent you're using (Claude, Codex, opencode, etc.):

> "explore the repo, read the readme and walk me through the demo section `First Demo: From Requirement To Verified AC` step by step and explain what is happening, I'm already running `demo-container-shell`"

## Is this Gherkin/BDD ?

You might think this is another BDD syntax or testing methodology.

It is not.

Gherkin structures executable examples. TDD structures the feedback loop between tests and implementation.

Coherence is a layer above.

We structure the relationship between a behavioral claim and the evidence that supports it.

It is a graph of requirements:

```text
┌──────────────────────┐
│ SPEC: Authentication │
└──────────┬───────────┘
           │ required_by
           ▼
┌──────────────────────┐       constrained_by       ┌──────────────────────┐
│  SPEC: Payment API   │ ─────────────────────────▶│ SPEC: PCI Compliance │
└──────────┬───────────┘                            └──────────────────────┘
           │ has
           ▼
┌────────────────────────────┐
│ AC: Rejects expired cards  │
└──────────┬─────────────────┘
           │ verified_by
           ▼
┌──────────────────────────────────────────┐
│ TEST: cargo test rejects_expired_cards   │
└──────────────────────────────────────────┘
```

`Payment API`, `Authentication`, and `PCI Compliance` are separate spec nodes. Their typed relationships explain how one outcome depends on or is constrained by another.

Acceptance criteria then connect those outcomes to implementation and executable evidence.

This is `Payment API` subgraph can be projected as a tree:

```text
SPEC: Payment API
  ├── depends_on → SPEC: Authentication
  ├── constrained_by → SPEC: PCI compliance
  └── has → AC: Rejects expired cards
               ├── implemented_by → backend/payment.rs
               ├── verified_by → cargo test rejects_expired_cards
               └── verified_by → payment.feature
```

From here, we can immediately jump to `payment.feature`, `reject_expired_cards`, or any connected spec and inspect surrounding context.

The same graph slice can also be materialized as an editable DSL:

```rust
coherence_slice! {
    changelist "payment-api-expired-cards" {
        spec "product/payment-api" {
            title: "Payment API"
            level: System
            status: Active

            links {
                depends_on "security/authentication"
                constrained_by "compliance/pci-dss"
            }

            ac "rejects-expired-cards" {
                title: "Rejects expired cards"
                intent: "An expired card is rejected with INVALID_CARD"
                risk: High
                concerns: [Correctness, Security]

                links {
                    implemented_by file "backend/payment.rs"
                    verified_by test "cargo test rejects_expired_cards"
                    verified_by feature "features/payment.feature"
                }
            }
        }

        context {
            spec "security/authentication" {
                title: "Authentication"
            }

            spec "compliance/pci-dss" {
                title: "PCI DSS compliance"
            }
        }
    }
}
```

This DSL is what engineers review before diving into the code in a pull request.

The same graph slice can be rendered as a simpler view for non-technical stakeholders. Design, GTM, HR, legal, security, and product teams can review the outcomes and claims that affect them without reading implementation details.

They are not approving the code, that's on you. They are confirming that the intended change is represented correctly.

Once that shared intent is agreed upon, engineers and agents can implement it. The graph provides the context, the why, and the expected behavior, making the resulting code easier to understand and review selectively.

The key trust boundary is `verified_by`.

A passing test is not enough. A human must confirm that the linked evidence actually verifies the acceptance criterion it claims to verify. After that relationship has been reviewed and signed off, automation can continuously report whether the claim still holds.

```text
stakeholders approve intent
→ engineers implement and review
→ humans validate the evidence link
→ automation verifies behavior repeatedly
```

Yes, this is still work. You still have to understand the system you are building.

Coherence _does not_ remove that responsibility. It makes that understanding explicit, structured, reviewable, and reusable, so the next person does not have to reconstruct it from code alone.

## Install On macOS Without Docker

This path installs the local bootstrap binary and uses a normal Dolt runtime on your Mac.

Prerequisites:

```bash
brew install dolt git
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Open a new shell after `rustup`, or load Cargo into the current one:

```bash
source "$HOME/.cargo/env"
```

Build and install `coherence-bootstrap` into `~/.local/bin`:

```bash
make install-local-force
```

Make sure your shell can find it:

```bash
export PATH="$HOME/.local/bin:$PATH"
coherence-bootstrap help
```

Initialize this checkout's catalog and start Dolt:

```bash
make tool bootstrap
make tool doctor
```

If `doctor` reports Dolt is not reachable, start it explicitly:

```bash
make tool dolt-start
make tool migrate
make tool doctor
```

Run the basic quality gate:

```bash
make tool run CARGO_TEST_ARGS="-- --test-threads=1"
```

After that you can use the same CLI/TUI flow as the demo container:

```bash
coherence-bootstrap spec list
coherence-bootstrap tui
```

Notes:

- `~/.local/bin` is not always on macOS `PATH`; add `export PATH="$HOME/.local/bin:$PATH"` to your shell profile if needed.
- Rust is needed for install/build and for the current Rust AC-test verification path.
- Dolt is required at runtime because the catalog is a Dolt SQL database.

## Load The Bootstrap Spec Catalog

The committed bootstrap spec export is here:

```text
.coherence/exports/bootstrap-specs.jsonl
```

Coherence does not use one global mutable spec database for every project. Each project gets its own logical Dolt database/catalog. When `DOLT_DB` is not set, the catalog name is derived from that project's `.coherence/project.toml` binding, including the project slug, a short hash tied to the project path, and the selected environment tier (`dev`, `test`, or `prod`).

That means the bootstrap specs and your demo app specs should normally live in different catalogs:

- Use this repository checkout when you want to inspect or modify the bootstrap catalog export.
- Use a separate demo/project directory when you want to try creating your own first spec and AC.
- Do not import `.coherence/exports/bootstrap-specs.jsonl` into a throwaway demo project unless you intentionally want that demo project's catalog to contain the bootstrap specs.
- Do not create throwaway demo specs in this repository's normal `dev` catalog unless you intend to export or clean them up afterward.

If you initialized a fresh catalog and want to load these specs into it, import the JSONL export:

```bash
coherence-bootstrap db import-jsonl \
  --env dev \
  --in .coherence/exports/bootstrap-specs.jsonl \
  --confirm
```

Then inspect or edit it:

```bash
coherence-bootstrap spec list
coherence-bootstrap tui
```

After adding or changing specs/ACs, write the catalog back to the export file:

```bash
coherence-bootstrap db export-jsonl \
  --env dev \
  --out .coherence/exports/bootstrap-specs.jsonl
```

If you want to reset the dev catalog to exactly what is in the export, truncate first and then import:

```bash
coherence-bootstrap db truncate --env dev --confirm
coherence-bootstrap db import-jsonl \
  --env dev \
  --in .coherence/exports/bootstrap-specs.jsonl \
  --confirm
```

## CodeScene CLI

This repo has thin wrappers for the modern CodeScene CLI (`cs`). Secrets stay in local `.env`; only `.env.example` is committed.

```bash
cp .env.example .env
```

Fill this value in `.env` for CodeScene Cloud/open-source projects:

```bash
CS_ACCESS_TOKEN=your-token
CS_PROJECT_ID=81147
```

For self-hosted CodeScene Enterprise only, also set:

```bash
CS_ONPREM_URL=https://your.codescene.example
```

Install or verify the CLI:

```bash
make codescene-install
```

Run delta analysis against `origin/master`:

```bash
make codescene-delta
```

Run only staged changes:

```bash
make codescene-delta-staged
```

Override the base branch/ref:

```bash
CODESCENE_BASE=origin/main make codescene-delta
```

The `cs` CLI is diff/file-oriented. For a full repository snapshot, use the CodeScene REST API wrapper:

```bash
make codescene-full
```

It triggers a full analysis when the API accepts it, then downloads the latest project analysis artifacts into an ignored local directory:

```text
.coherence/review/codescene/<timestamp>/
```

The snapshot includes `issues.json`, `files-by-code-health.json`, `technical-debt.json`, `hotspots.json`, `components.json`, and analysis metadata.

## First Demo: From Requirement To Verified AC

This path is for a person seeing Coherence for the first time. It is also written so an agent can follow it literally.

Use a separate throwaway project directory for this demo if you are not intentionally editing the bootstrap specs. Catalog identity is per project path/binding, so a demo project gets its own Dolt logical database instead of mixing with this repository's bootstrap catalog.

The demo container contains `coherence-bootstrap`, Dolt, Git, Bash, the Rust toolchain needed by the current Rust AC-test verifier, and a copy of the committed bootstrap spec export. Start it from this repository:

```bash
make demo-container-shell
```

You should now be inside the container at `/root/git/demo`. The entrypoint started `dolt sql-server`, and `make demo-container-shell` ran `coherence-demo-setup` before opening the shell.

The setup creates two separate project directories:

- `/root/git/coherence-bootstrap`: a Coherence project with the committed bootstrap specs imported into its `dev` catalog.
- `/root/git/demo`: a separate minimal Coherence/Rust project for your own first spec and AC.

These are two different project paths with two different `.coherence/project.toml` bindings, so they resolve to different Dolt logical databases when `DOLT_DB` is unset. The TUI discovers projects by path under `~/git`, so running `coherence-bootstrap tui` from anywhere in the container should show both.

The demo project includes a tiny Rust package named `coherence-core-db-bootstrap` because the current MVP AC materializer links Rust AC tests with `cargo test -p coherence-core-db-bootstrap <test_name>`.

If you want `/root/git/demo` to be a host directory instead of an ephemeral in-container project, pass it explicitly:

```bash
DEMO_WORKSPACE=/path/to/my/project make demo-container-shell
```

1. Confirm both catalogs are ready.

```bash
cd /root/git/coherence-bootstrap
coherence-bootstrap spec list
cd /root/git/demo
coherence-bootstrap doctor
```

If you started the image manually with raw `docker run ... bash`, run this first:

```bash
coherence-demo-setup
```

2. Turn one requirement into a spec.

Requirement: "The demo app prints a human-readable greeting."

```bash
coherence-bootstrap spec add \
  --id SPEC-demo-greeting \
  --slug product/demo-greeting \
  --title "Demo Greeting" \
  --level product \
  --status draft \
  --description "The demo app exposes a simple greeting behavior."
```

3. Turn the testable claim into an acceptance criterion.

```bash
coherence-bootstrap ac add \
  --id AC--demo-greeting-prints-message \
  --spec-id SPEC-demo-greeting \
  --slug prints-message \
  --title "prints greeting message" \
  --intent "Running the demo greeting behavior produces Hello, Coherence!"
```

4. Inspect the catalog.

```bash
coherence-bootstrap spec list
coherence-bootstrap ac list --spec-id SPEC-demo-greeting
```

5. Open the TUI and edit the spec.

```bash
coherence-bootstrap tui
```

In the TUI:

- Press `Enter` on the project.
- Press `Enter` on `dev`.
- Use arrows to select `Product`, press `Enter` to expand it.
- Select `SPEC-demo-greeting`.
- Press `e` to enter edit mode.
- Press `s` to cycle status, or `l` to cycle level.
- Press `Enter` to save.
- Press `q` to quit.

6. Materialize the Rust AC test skeleton.

```bash
coherence-bootstrap ac-tests materialize-rust \
  --workspace /root/git/demo \
  --ac-id AC--demo-greeting-prints-message
```

This creates one file:

```text
tests/ac_prints-message.rs
```

It also records a `verified_by` codeintel link for that AC. The current MVP verifier uses the generated command `cargo test -p coherence-core-db-bootstrap validates_prints_message`.

7. Write the first test.

Open `tests/ac_prints-message.rs` and replace the generated `todo!(...)` with a real assertion:

```rust
//! AC: AC--demo-greeting-prints-message
//! Generated by coherence-core-db MVP AC test layout.

#[test]
fn validates_prints_message() {
    let greeting = "Hello, Coherence!";
    assert_eq!(greeting, "Hello, Coherence!");
}
```

8. Let Coherence verify the AC through the catalog link.

```bash
coherence-bootstrap verify-ac AC--demo-greeting-prints-message
```

Expected outcome: `OVERALL passed` and a `LINK ... passed` row. You can also verify every AC under the spec:

```bash
coherence-bootstrap verify-spec SPEC-demo-greeting
```

9. Optional cleanup for this demo file.

```bash
rm -f tests/ac_prints-message.rs
```

# WIP Coherence Specs aka "Normalized Spec Tree Decisions v3"

This section is a working snapshot of the bootstrap spec catalog recovered from the existing codebase. The live/exportable catalog lives in `.coherence/exports/bootstrap-specs.jsonl`; this README section explains the recovery shape and current tree at a human-readable level.

The reverse-engineering goal was not to document every function. The goal was to answer: "what behavior does this codebase already promise, and at which abstraction level should each promise live?" We used a taxonomy first, then routed discovered behavior into that taxonomy.

The recovery flow was roughly:

- Build an inventory from the codebase: files, symbols, commands, tests, persistence surfaces, and observable behaviors.
- Convert that inventory into a ledger of candidate specs and candidate ACs.
- Apply the taxonomy below: `FOUNDATION -> MODULE -> COMPONENT -> SYSTEM -> PRODUCT`.
- Group flat findings into coherent specs instead of one spec per code detail.
- Promote user-visible or contract-level claims into ACs.
- Mark test infrastructure or implementation-only details as evidence-only/demoted instead of product promises.
- Keep higher-level specs focused on their own level: a product AC should not re-test a foundation DB invariant if that invariant already belongs to a foundation AC.

This gives a useful behavioral inventory without requiring every reviewer to read the whole codebase. The process is intentionally repeatable: the same codebase, the same taxonomy, and the same routing rules should converge toward a similar spec tree.

Current committed catalog export:

```text
.coherence/exports/bootstrap-specs.jsonl
```

**Source:** `normalized_inventory.md` / ledger files
**Taxonomy:** 5-level (PRODUCT → SYSTEM → MODULE → COMPONENT → FOUNDATION)
**Purpose:** Exhaustive routing of every ledger row. No row disappears.
**Rows:** 252 total | 219 promoted_to_ac | 27 evidence_only | 4 demoted | 2 moved_to_lower_level

---

## Principle

```
FOUNDATION  → domain models + infrastructure contracts (no product UX)
MODULE      → bounded capabilities using FOUNDATION models
COMPONENT   → concrete adapters (parser, repository, shell-runner, tree renderer)
SYSTEM      → end-to-end processes (authoring, verification, evidence, isolation)
PRODUCT     → user-facing surfaces (CLI commands, TUI screens)
```

**Rule:** Higher level does NOT re-verify lower-level invariants.

---

## Level Distribution

| Level | Rows |
|-------|------|
| PRODUCT | 101 |
| FOUNDATION | 83 |
| SYSTEM | 55 |
| COMPONENT | 13 |
| MODULE | 0 |

---

## Spec Tree (v3)

```
PRODUCT
├── product/cli/spec              [9 rows: 9 ACs, 0 evidence, 0 demoted]
│   ├── spec-add (4): slug required, title required, description optional, returns identity
│   └── spec-list-show (5): no args, list with fields, inspect by id, not-found error, field display
│
├── product/cli/ac               [10 rows: 10 ACs, 0 evidence, 0 demoted]
│   ├── ac-add (5): spec must exist, title required, intent optional, slug optional, returns identity
│   └── ac-list-show (5): scoped to spec, list with fields, inspect by id, not-found error, field display
│
├── product/cli/verify           [10 rows: 10 ACs, 0 evidence, 0 demoted]
│   ├── verify-ac (5): positional/flag id, exit reflects links, aggregate output, no-links message, LINK rows
│   └── verify-spec (5): positional/flag id, aggregate counts, per-AC outcome, not-found error, consistent output
│
├── product/cli/ac-tests         [11 rows: 11 ACs, 0 evidence, 0 demoted]
│   ├── materialize-check (7): generate skeletons, path safety, check-rust, scope by id, workspace discovery, reports, codeintel alignment
│   └── test-file-layout (4): flat file per AC, slug→Rust name, skeleton #[test] fn, deterministic cargo command
│
├── product/cli/project          [8 rows: 8 ACs, 0 evidence, 0 demoted]
│   └── project (8): init creates manifest and is idempotent, binds once, does not overwrite, --force-rebind, --workspace, requires git, reset repair, preflight check
│
├── product/cli/db               [9 rows: 9 ACs, 0 evidence, 0 demoted]
│   └── db (9): list logical dbs, resolve params from env, enumerate MySQL dbs, db-ping reachable, db-ping validates preflight, db-ping success, drop disposable, drop no-op in repo-local, drop refuses non-disposable
│
├── product/cli/doctor           [6 rows: 6 ACs, 0 evidence, 0 demoted]
│   └── doctor (6): workflow backend, COHERENCE_ENV validity, effective catalog, manifest binding, DOLT_DB override, git worktree
│
├── product/cli/evidence-sample   [4 rows: 4 ACs, 0 evidence, 0 demoted]
│   └── evidence-sample (4): creates run dir layout, workspace+run-id targeting, auto-generates run-id
│
├── product/cli/tui              [1 rows: 1 ACs, 0 evidence, 0 demoted]
│   └── tui-launch (1): launches TUI binary
│
├── product/cli/help             (see doctor rows)
│
├── product/tui/navigation        [22 rows: 22 ACs, 0 evidence, 0 demoted]
│   ├── tree (7): grouped by level, expand/collapse, item selection, preview update, viewport scroll, expand markers, box-drawing detail
│   ├── screens (5): three-row layout, project picker, env picker, spec browser, back reverses stack
│   ├── navigation (5): keyboard bindings, three screens stateful, environment selection (dev/test/prod), auto-discovers projects under $HOME/git, event loop processes actions
│   └── rendering (5): color theme, visible-only rendering, status icons, selection highlighting, TUI connects with project dir restore
│
├── product/tui/editing           [8 rows: 8 ACs, 0 evidence, 0 demoted]
│   ├── draft (4): draft tracks original+pending, unsaved detection, field validation, single-key shortcuts
│   └── effects (4): spec/AC persist, editor invocation, graph refresh, effects execute
│
└── product/tui/verification       [3 rows: 3 ACs, 0 evidence, 0 demoted]
    └── verification (3): verify single AC and display icon, verify all specs, shortcuts only in browse mode

SYSTEM
├── system/process/verification   [10 rows: 10 ACs, 0 evidence, 0 demoted]
│   └── verification (10): executes verified_by links, skips non-executable, sh subprocess, output truncation 240chars/512KB, persist to store, nonzero on fail, spec aggregate, unknown spec error, aggregate counts+freshness, per-link outcomes
│
├── system/process/test-isolation  [15 rows: 10 ACs, 5 evidence, 0 demoted]
│   ├── isolation-policy (8): requires COHERENCE_DB_PROFILE=test, permits when active+disposable, refuses when unset, canonical slug refused, coherence_test_ prefix always permitted, allowlist bypass, dev tier protection, COHERENCE_TEST_WORLD assertion
│   └── smoke-test (2): m0-smoke minimal smoke, m1-spec-smoke refuses canonical catalog → evidence_only
│       preservation-marker (1): isolated wrapper prints preservation marker on failure → evidence_only
│
├── system/process/ac-authoring    [2 rows: 2 ACs, 0 evidence, 0 demoted]
│   └── subcommand (2): requires add/list/show, rejects unknown
│
├── system/process/spec-authoring  [2 rows: 2 ACs, 0 evidence, 0 demoted]
│   └── subcommand (2): requires add/list/show, rejects unknown
│
├── system/process/ac-test-materialization [4 rows: 4 ACs, 0 evidence, 0 demoted]
│   └── materialize-invariants (4): deterministic codeintel IDs, ID derivation from AC+location, MySQL VARCHAR(191) fit, cycle-safe hierarchy walk
│
├── system/process/evidence-capture [2 rows: 2 ACs, 0 evidence, 0 demoted]
│   └── evidence-persistence (2): run-id persists envelopes, best-effort never fails command
│
├── system/test/world              [17 rows: 0 ACs, 17 evidence, 0 demoted]
│   └── test-primitives (17): all rows marked evidence_only — DoltWorld, Scaffold, Services, Recipe, EnvGuard, CommandRequest, world run, evidence persistence → test infrastructure, not product promise
│
└── system/cli/operator-tools      [3 rows: 3 ACs, 0 evidence, 0 demoted]
    └── operator-tools (3): (remaining rows not matched above — default operator tooling)

COMPONENT
├── component/cli/parser          [5 rows: 5 ACs, 0 evidence, 0 demoted]
│   └── argument-parsing (5): --key value flags, empty/missing/dash-start errors, multi-flag collection, single-flag error-on-multiple, multi-flag empty-absent
│
├── component/cli/router          [2 rows: 2 ACs, 0 evidence, 0 demoted]
│   └── routing (2): argv[1] dispatch, unknown command exit 64
│
├── component/repository/spec-store [6 rows: 6 ACs, 0 evidence, 0 demoted]
│   └── spec-persistence (6): upsert/get/list specs, load spec graph bulk
│
└── component/repository/ac-store  (see acceptance-criteria model rows)

FOUNDATION
├── foundation/domain/model/specs  [8 rows: 8 ACs, 0 evidence, 0 demoted]
│   ├── model-invariants (3): level enum (product/system/module), status enum (4-value), slug derivation from id
│   ├── model-defaults (2): level defaults to module, status defaults to draft
│   └── persistence (3): upsert/get/list, spec relations upsert/list
│
├── foundation/domain/model/acceptance-criteria [12 rows: 12 ACs, 0 evidence, 0 demoted]
│   ├── model-invariants (5): review_mode enum (manual/automated/hybrid), risk_level enum (low/medium/high/critical), slug uniqueness per spec, concerns 5-value enum, slug non-empty
│   ├── model-defaults (4): manual review, medium risk, slug from id, no concerns
│   └── persistence (3): upsert with concerns replacement, get by id with concerns, list by spec
│
├── foundation/domain/model/acceptance-criterion-concerns [1 rows: 1 ACs, 0 evidence, 0 demoted]
│   └── concerns (1): AC may carry zero or more concern kinds
│
├── foundation/domain/model/spec-relations [embedded in specs persistence above]
│
├── foundation/domain/model/codeintel-code-locations [4 rows: 4 ACs, 0 evidence, 0 demoted]
│   └── codeintel-location (4): upsert by id, kind enum (file/test/table/manual_gap), kind distinguishes test_file from test_command
│
├── foundation/domain/model/codeintel-ac-links [6 rows: 6 ACs, 0 evidence, 0 demoted]
│   └── codeintel-link (6): upsert links, list by AC with locations, fail on dangling location, relation kind enum (implementation/verification/formalization/refinement/tracking), relation kind 3-value enum, joined retrieval
│
├── foundation/domain/model/ac-verification-latest [7 rows: 7 ACs, 0 evidence, 0 demoted]
│   └── verification-latest (7): cache each run replaces prior, aggregate counts+freshness, load latest, load per-link, tolerate missing migration table, exit code + skip reason storage
│
├── foundation/infra/dolt/connectivity [16 rows: 16 ACs, 0 evidence, 0 demoted]
│   ├── env-resolution (5): full config from env vars, socket/TCP fallback, COHERENCE_ENV validation, COHERENCE_PROJECT_SLUG from manifest, effective db name resolution
│   ├── preflight (3): catalog preflight enforces git+manifest+binding, error includes command context, server readiness no-op query
│   ├── socket-tcp (2): prefers unix socket, transparent TCP fallback
│   └── server-ops (3): SHOW DATABASES without catalog selection, migrate provisions all tiers, explicit DOLT_DB bypasses tier provisioning
│
├── foundation/infra/dolt/catalog-naming [6 rows: 6 ACs, 0 evidence, 0 demoted]
│   └── catalog-naming (6): identifier sanitization, git hash 4-char, legacy slug+hash, slug_hash_env truncation, COHERENCE_ENV case-insensitive defaults dev, COHERENCE_ENV controls tier
│
├── foundation/infra/dolt/migrations [9 rows: 9 ACs, 0 evidence, 0 demoted]
│   └── migrations (9): apply all Refinery migrations, validate preflight, report catalog+count, exit 0 on clean, provision both modules, independent schema histories, socket-first connection, combined count report, spec commands require migrated schema
│
├── foundation/infra/filesystem/project-manifest [5 rows: 5 ACs, 0 evidence, 0 demoted]
│   └── manifest (5): git root discovery without git subprocess, TOML at .coherence/project.toml, write and persist, schema v2+ required for project_hash, best-effort read returns None
│
└── foundation/infra/filesystem/evidence-layout [9 rows: 9 ACs, 0 evidence, 0 demoted]
    └── evidence-layout (9): deterministic .coherence/runs/<run-id>/, on-demand dir creation, metadata/run.json manifest, large outputs as artifacts, SnapshotEnvelope per observation, read by observation id, future-compatible pointer stub, SnapshotEnvelope schema (identity/kind/object/payload/SHA-256)
```

---

## Actions Summary

| Action | Count | Description |
|--------|-------|-------------|
| promoted_to_ac | 219 | Row becomes an AC in the final spec catalog |
| evidence_only | 27 | Row is evidence of behavior but not an AC |
| demoted | 4 | Row is internal implementation detail |
| moved_to_lower_level | 2 | Row absorbed into another spec (ID generation) |

### Demoted rows (4)

| Spec | AC | Reason |
|------|----|--------|
| product/ac-test-materialization | materialized test files get deterministic codeintel IDs derived from their content | Internal ID composition |
| product/ac-test-materialization | verified_by links get deterministic IDs derived from AC and location identity | Internal ID derivation |
| product/ac-test-materialization | codeintel IDs fit the MySQL VARCHAR(191) primary key constraint | Internal constraint |
| product/ac-test-materialization | spec hierarchy walk is deterministic and cycle-safe | Implementation detail |

### Moved to lower level (2)

| From | To | AC | Reason |
|------|----|----|--------|
| system/ac-identity | foundation/domain/model/acceptance-criteria | CLI can generate an AC id | ID generation is part of AC model lifecycle |
| system/spec-identity | foundation/domain/model/specs | CLI can generate a spec id | ID generation is part of spec model lifecycle |

### Evidence-only rows (27)

- **system/test/world**: 17 rows — all test infrastructure primitives (DoltWorld, Scaffold, Services, Recipe, EnvGuard, world run, evidence) — marked `evidence_type=test-infra`
- **system/process/test-isolation** smoke tests: 5 rows — m0-smoke, m1-spec-smoke, preservation marker — marked `evidence_type=smoke`
- **system/cli/operator-tools** smoke test rows: 5 rows — help/doctor/project_init from cli_smoke.rs — marked `evidence_type=smoke`

---

## Final Spec Tree Summary

```
PRODUCT (11 specs)
├── product/cli/spec
├── product/cli/ac
├── product/cli/verify
├── product/cli/ac-tests
├── product/cli/project
├── product/cli/db
├── product/cli/doctor
├── product/cli/evidence-sample
├── product/cli/tui
├── product/tui/navigation
├── product/tui/editing
└── product/tui/verification

SYSTEM (7 specs)
├── system/process/verification
├── system/process/test-isolation
├── system/process/ac-authoring
├── system/process/spec-authoring
├── system/process/ac-test-materialization
├── system/process/evidence-capture
└── system/test/world

COMPONENT (3 specs)
├── component/cli/parser
├── component/cli/router
└── component/repository/spec-store

MODULE (0 specs) — none emerge from bootstrap inventory

FOUNDATION (7 specs)
├── foundation/domain/model/specs
├── foundation/domain/model/acceptance-criteria
├── foundation/domain/model/acceptance-criterion-concerns
├── foundation/domain/model/codeintel-code-locations
├── foundation/domain/model/codeintel-ac-links
├── foundation/domain/model/ac-verification-latest
├── foundation/infra/dolt/connectivity
├── foundation/infra/dolt/catalog-naming
├── foundation/infra/dolt/migrations
├── foundation/infra/filesystem/project-manifest
└── foundation/infra/filesystem/evidence-layout

TOTAL: 28 specs
```

---

## Proposed DB Write Order

```
Phase 1 — FOUNDATION (no dependencies on other bootstrap specs)
  foundation/domain/model/specs
  foundation/domain/model/acceptance-criteria
  foundation/domain/model/acceptance-criterion-concerns
  foundation/domain/model/spec-relations
  foundation/domain/model/codeintel-code-locations
  foundation/domain/model/codeintel-ac-links
  foundation/domain/model/ac-verification-latest
  foundation/infra/dolt/connectivity
  foundation/infra/dolt/catalog-naming
  foundation/infra/dolt/migrations
  foundation/infra/filesystem/project-manifest
  foundation/infra/filesystem/evidence-layout

Phase 2 — COMPONENT
  component/cli/parser
  component/cli/router
  component/repository/spec-store

Phase 3 — SYSTEM
  system/process/ac-authoring
  system/process/spec-authoring
  system/process/verification
  system/process/ac-test-materialization
  system/process/evidence-capture
  system/process/test-isolation
  system/test/world  (evidence-only rows, not ACs in catalog)

Phase 4 — PRODUCT
  product/cli/spec
  product/cli/ac
  product/cli/verify
  product/cli/ac-tests
  product/cli/project
  product/cli/db
  product/cli/doctor
  product/cli/evidence-sample
  product/cli/tui
  product/tui/navigation
  product/tui/editing
  product/tui/verification
```

---

_Generated: 252 ledger rows → 219 ACs + 27 evidence + 4 demoted + 2 absorbed across 28 final specs._
---

## Full Routing Table (252 rows)

| # | Original Spec | AC (truncated) | Level | Final Spec | Group | Action | Reason |
|---|-------------|----------------|-------|------------|-------|--------|--------|
| 1 | system/cli/ac-command | ac command requires an explicit add/list/show subcomman | SYSTEM | system/process/ac-authoring | subcommand-dispatch | promoted_to_ac | CLI subcommand dispatch belongs to ac-authori |
| 2 | system/cli/ac-command | ac command rejects unknown subcommands with expected ch | SYSTEM | system/process/ac-authoring | subcommand-dispatch | promoted_to_ac | CLI subcommand dispatch belongs to ac-authori |
| 3 | product/ac-authoring | ACs are created against an existing spec; unknown spec- | PRODUCT | product/cli/ac | ac-add | promoted_to_ac | AC authoring UX is product/cli/ac |
| 4 | product/ac-authoring | operator must provide a human title when authoring an a | PRODUCT | product/cli/ac | ac-add | promoted_to_ac | AC authoring UX is product/cli/ac |
| 5 | product/ac-authoring | operator may omit intent description when creating an A | PRODUCT | product/cli/ac | ac-add | promoted_to_ac | AC authoring UX is product/cli/ac |
| 6 | product/ac-authoring | operator may provide a stable slug when authoring an AC | PRODUCT | product/cli/ac | ac-add | promoted_to_ac | AC authoring UX is product/cli/ac |
| 7 | system/ac-model | new ACs have a review mode and default to manual when o | FOUNDATION | foundation/domain/model/acceptance-criteria | model-defaults | promoted_to_ac | Model defaults are foundation/domain behavior |
| 8 | system/ac-model | new ACs have a risk level and default to medium when om | FOUNDATION | foundation/domain/model/acceptance-criteria | model-defaults | promoted_to_ac | Model defaults are foundation/domain behavior |
| 9 | system/ac-model | ACs may carry zero or more concern kinds | FOUNDATION | foundation/domain/model/acceptance-criterion-concerns | concerns-model | promoted_to_ac | Concerns model is a separate domain concept |
| 10 | system/ac-identity | CLI can generate an AC id when the operator does not pr | FOUNDATION | foundation/domain/model/acceptance-criteria | id-generation | moved_to_lower_level | ID generation is part of AC model lifecycle |
| 11 | product/ac-authoring | successful AC creation returns the created identity to  | PRODUCT | product/cli/ac | ac-add | promoted_to_ac | AC authoring UX is product/cli/ac |
| 12 | product/ac-browsing | AC list is always scoped to a spec via required --spec- | PRODUCT | product/cli/ac | ac-list-show | promoted_to_ac | AC browsing is part of product/cli/ac |
| 13 | product/ac-browsing | operator can list ACs for a spec with identity, title,  | PRODUCT | product/cli/ac | ac-list-show | promoted_to_ac | AC browsing is part of product/cli/ac |
| 14 | product/ac-browsing | operator can inspect an AC by id through positional or  | PRODUCT | product/cli/ac | ac-list-show | promoted_to_ac | AC browsing is part of product/cli/ac |
| 15 | product/ac-browsing | AC inspection fails clearly for unknown ids | PRODUCT | product/cli/ac | ac-list-show | promoted_to_ac | AC browsing is part of product/cli/ac |
| 16 | product/ac-browsing | operator can inspect persisted intent, review mode, ris | PRODUCT | product/cli/ac | ac-list-show | promoted_to_ac | AC browsing is part of product/cli/ac |
| 17 | system/codeintel-model | codeintel code locations can be created or updated via  | FOUNDATION | foundation/domain/model/codeintel-code-locations | codeintel-location-model | promoted_to_ac | Code location model is a separate domain conc |
| 18 | system/codeintel-model | codeintel code locations can be retrieved individually  | FOUNDATION | foundation/domain/model/codeintel-code-locations | codeintel-location-model | promoted_to_ac | Code location model is a separate domain conc |
| 19 | system/codeintel-model | AC-to-codeintel links can be created or updated via ups | FOUNDATION | foundation/domain/model/codeintel-ac-links | codeintel-link-model | promoted_to_ac | Codeintel links are a separate domain concept |
| 20 | system/codeintel-model | all codeintel links for a given AC can be listed with t | FOUNDATION | foundation/domain/model/codeintel-ac-links | codeintel-link-model | promoted_to_ac | Codeintel links are a separate domain concept |
| 21 | system/codeintel-model | listing links for an AC fails clearly when a code locat | FOUNDATION | foundation/domain/model/codeintel-ac-links | codeintel-link-model | promoted_to_ac | Codeintel links are a separate domain concept |
| 22 | system/codeintel-model | codeintel code locations carry a kind from a fixed enum | FOUNDATION | foundation/domain/model/codeintel-code-locations | codeintel-location-model | promoted_to_ac | Code location model is a separate domain conc |
| 23 | system/codeintel-model | AC-to-codeintel links carry a relation kind from a fixe | FOUNDATION | foundation/domain/model/codeintel-ac-links | codeintel-link-model | promoted_to_ac | Codeintel links are a separate domain concept |
| 24 | product/ac-test-materialization | materialized test files get deterministic codeintel IDs | SYSTEM | system/process/ac-test-materialization | materialize-invariants | demoted | Internal ID composition - implementation deta |
| 25 | product/ac-test-materialization | verified_by links get deterministic IDs derived from AC | SYSTEM | system/process/ac-test-materialization | materialize-invariants | demoted | Internal ID composition - implementation deta |
| 26 | product/ac-test-materialization | codeintel IDs fit the MySQL VARCHAR(191) primary key co | SYSTEM | system/process/ac-test-materialization | materialize-invariants | demoted | Internal ID composition - implementation deta |
| 27 | product/ac-test-materialization | each AC yields a flat test file at tests/ac_<slug>.rs r | PRODUCT | product/cli/ac-tests | test-file-layout | promoted_to_ac | Test file layout is product concern |
| 28 | product/ac-test-materialization | AC slugs are converted to valid Rust test function name | PRODUCT | product/cli/ac-tests | test-file-layout | promoted_to_ac | Test file layout is product concern |
| 29 | product/ac-test-materialization | materialized test files contain a skeleton #[test] fn w | PRODUCT | product/cli/ac-tests | test-file-layout | promoted_to_ac | Test file layout is product concern |
| 30 | product/ac-test-materialization | each materialized test file has a deterministic cargo t | PRODUCT | product/cli/ac-tests | test-file-layout | promoted_to_ac | Test file layout is product concern |
| 31 | product/ac-test-materialization | spec hierarchy walk is deterministic and cycle-safe | SYSTEM | system/process/ac-test-materialization | materialize-invariants | demoted | Internal ID composition - implementation deta |
| 32 | product/ac-test-materialization | operator can generate missing Rust AC test skeletons fr | PRODUCT | product/cli/ac-tests | materialize-check | promoted_to_ac | AC materialize/check is product/cli/ac-tests |
| 33 | product/ac-test-materialization | each materialize-rust run keeps the codeintel DB aligne | PRODUCT | product/cli/ac-tests | materialize-check | promoted_to_ac | AC materialize/check is product/cli/ac-tests |
| 34 | product/ac-test-materialization | materialize-rust refuses to write outside the tests/ di | PRODUCT | product/cli/ac-tests | materialize-check | promoted_to_ac | AC materialize/check is product/cli/ac-tests |
| 35 | product/ac-test-materialization | check-rust verifies that every expected AC test file ex | PRODUCT | product/cli/ac-tests | materialize-check | promoted_to_ac | AC materialize/check is product/cli/ac-tests |
| 36 | product/ac-test-materialization | operator can scope materialize or check to a single AC  | PRODUCT | product/cli/ac-tests | materialize-check | promoted_to_ac | AC materialize/check is product/cli/ac-tests |
| 37 | product/ac-test-materialization | ac-tests auto-discovers the workspace root by walking u | PRODUCT | product/cli/ac-tests | materialize-check | promoted_to_ac | AC materialize/check is product/cli/ac-tests |
| 38 | product/ac-test-materialization | materialize-rust reports which files were created, alre | PRODUCT | product/cli/ac-tests | materialize-check | promoted_to_ac | AC materialize/check is product/cli/ac-tests |
| 39 | system/codeintel-model | verification run results for an AC are persisted as a c | FOUNDATION | foundation/domain/model/ac-verification-latest | verification-storage | promoted_to_ac | Verification latest storage is a separate dom |
| 40 | system/codeintel-model | verification results include aggregate counts and a fre | FOUNDATION | foundation/domain/model/ac-verification-latest | verification-storage | promoted_to_ac | Verification latest storage is a separate dom |
| 41 | system/codeintel-model | the latest verification result for an AC can be loaded  | FOUNDATION | foundation/domain/model/ac-verification-latest | verification-storage | promoted_to_ac | Verification latest storage is a separate dom |
| 42 | system/codeintel-model | verification latest query tolerates missing migration t | FOUNDATION | foundation/domain/model/ac-verification-latest | verification-storage | promoted_to_ac | Verification latest storage is a separate dom |
| 43 | system/codeintel-model | the latest per-link verification outcomes for an AC can | FOUNDATION | foundation/domain/model/ac-verification-latest | verification-storage | promoted_to_ac | Verification latest storage is a separate dom |
| 44 | system/codeintel-model | link latest query tolerates missing migration table | FOUNDATION | foundation/domain/model/ac-verification-latest | verification-storage | promoted_to_ac | Verification latest storage is a separate dom |
| 45 | system/codeintel-model | verification link outcomes store full status with exit  | FOUNDATION | foundation/domain/model/ac-verification-latest | verification-storage | promoted_to_ac | Verification latest storage is a separate dom |
| 46 | product/verification | verifying an AC executes all verified_by linked test co | SYSTEM | system/process/verification | verification-process | promoted_to_ac | Verification process is a system concern |
| 47 | product/verification | only codeintel links of kind verified_by with test_file | SYSTEM | system/process/verification | verification-process | promoted_to_ac | Verification process is a system concern |
| 48 | product/verification | a verified_by link without an executable test_command i | SYSTEM | system/process/verification | verification-process | promoted_to_ac | Verification process is a system concern |
| 49 | product/verification | verification commands run in a sh subprocess with null  | SYSTEM | system/process/verification | verification-process | promoted_to_ac | Verification process is a system concern |
| 50 | product/verification | long verification outputs are summarized to 240 chars a | SYSTEM | system/process/verification | verification-process | promoted_to_ac | Verification process is a system concern |
| 51 | product/verification | each verification run is persisted to the ac_verificati | SYSTEM | system/process/verification | verification-process | promoted_to_ac | Verification process is a system concern |
| 52 | product/verification | verify-ac returns nonzero exit code when any verified_b | SYSTEM | system/process/verification | verification-process | promoted_to_ac | Verification process is a system concern |
| 53 | product/verification | verify-spec aggregates the verification outcome of ever | SYSTEM | system/process/verification | verification-process | promoted_to_ac | Verification process is a system concern |
| 54 | product/verification | verify-spec fails clearly for unknown spec ids | SYSTEM | system/process/verification | verification-process | promoted_to_ac | Verification process is a system concern |
| 55 | product/verification | verify-spec exits nonzero if any constituent AC verific | SYSTEM | system/process/verification | verification-process | promoted_to_ac | Verification process is a system concern |
| 56 | system/cli/argument-parsing | the CLI entry point dispatches to appropriate command h | COMPONENT | component/cli/router | command-dispatch | promoted_to_ac | Command routing is a component responsibility |
| 57 | system/cli/argument-parsing | unknown commands produce exit code 64 with an error mes | COMPONENT | component/cli/router | unknown-command | promoted_to_ac | Unknown command behavior is a router concern |
| 58 | system/cli/argument-parsing | CLI argument parsing supports --key value flags and pos | COMPONENT | component/cli/parser | argument-parsing | promoted_to_ac | Argument parsing is a shared component |
| 59 | system/cli/argument-parsing | argument parsing fails clearly for empty flags, missing | COMPONENT | component/cli/parser | argument-parsing | promoted_to_ac | Argument parsing is a shared component |
| 60 | system/cli/argument-parsing | the same flag can appear multiple times and values are  | COMPONENT | component/cli/parser | argument-parsing | promoted_to_ac | Argument parsing is a shared component |
| 61 | system/cli/argument-parsing | single-flag retrieval fails when a flag appears more th | COMPONENT | component/cli/parser | argument-parsing | promoted_to_ac | Argument parsing is a shared component |
| 62 | system/cli/argument-parsing | multi-flag retrieval returns all values in order or emp | COMPONENT | component/cli/parser | argument-parsing | promoted_to_ac | Argument parsing is a shared component |
| 63 | system/operator-tooling | help command surfaces all key workflow entry points, DB | SYSTEM | system/cli/operator-tools | help-command | evidence_only | Smoke test rows - evidence of operator toolin |
| 64 | system/operator-tooling | doctor command surfaces workflow backend, DB policy, an | SYSTEM | system/cli/operator-tools | help-command | evidence_only | Smoke test rows - evidence of operator toolin |
| 65 | system/operator-tooling | project init creates a complete manifest on first run a | SYSTEM | system/cli/operator-tools | help-command | evidence_only | Smoke test rows - evidence of operator toolin |
| 66 | system/test-infrastructure | test infrastructure can save and restore environment va | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 67 | system/test-infrastructure | E2eRecipe provides a fluent builder for test environmen | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 68 | system/test-infrastructure | E2eEnvironment produces environment variables to connec | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 69 | system/test-infrastructure | Scaffold provides a temporary filesystem root for test  | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 70 | system/test-infrastructure | Scaffold can initialize a git repository in the test fi | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 71 | system/test-infrastructure | Services coordinates starting multiple services with he | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 72 | system/test-infrastructure | test world supports filesystem scaffold, command-only,  | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 73 | system/test-infrastructure | AC test commands are represented as CommandRequest with | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 74 | system/test-infrastructure | world runs shell commands with configurable working dir | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 75 | system/test-infrastructure | test evidence records command output and file snapshots | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 76 | system/test-infrastructure | evidence can be persisted to disk as JSON plus file sna | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 77 | system/test-infrastructure | AC tests run within a world with evidence captured for  | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 78 | system/db/connectivity | connection config can be fully resolved from environmen | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 79 | system/db/connectivity | socket path defaults to XDG_RUNTIME_DIR/coherence/dolt. | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 80 | system/db/connectivity | TCP port resolution follows a deterministic fallback ch | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 81 | system/db/connectivity | connection config fails clearly when COHERENCE_ENV has  | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 82 | system/db/connectivity | COHERENCE_PROJECT_SLUG is derived from the manifest whe | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 83 | system/db/connectivity | the effective database name follows: explicit DOLT_DB w | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 84 | system/db/connectivity | bound project_hash enables deterministic catalog naming | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 85 | system/db/connectivity | DOLT_DB override is detectable as a signal that an expl | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 86 | system/db/connectivity | catalog preflight enforces git root, readable manifest, | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 87 | system/db/connectivity | preflight error messages include the command context th | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 88 | system/db/connectivity | connection prefers unix socket and transparently falls  | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 89 | system/db/connectivity | server-level queries like SHOW DATABASES work without s | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 90 | system/db/connectivity | server readiness can be verified with a no-op query tha | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 91 | system/db/connectivity | migrate provisions all tier catalogs (_dev, _test, _pro | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 92 | system/db/connectivity | explicit DOLT_DB creates only that single catalog, bypa | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 93 | system/db/catalog-bootstrap | smoke test fixtures are normalized with defaults when p | SYSTEM | system/process/test-isolation | smoke-test | evidence_only | Smoke test evidence - test infrastructure, no |
| 94 | system/db/connectivity | user-scoped is the default mode when no project.toml or | FOUNDATION | foundation/infra/dolt/connectivity | connectivity-contract | promoted_to_ac | Dolt connectivity is infrastructure |
| 95 | system/db/connectivity | operator can list all logical databases on the Dolt ser | PRODUCT | product/cli/db | db-commands | promoted_to_ac | db-ping and db-list-databases are CLI operato |
| 96 | system/db/connectivity | db-list-databases resolves connection parameters from D | PRODUCT | product/cli/db | db-commands | promoted_to_ac | db-ping and db-list-databases are CLI operato |
| 97 | system/db/connectivity | operator can enumerate all MySQL database names on the  | PRODUCT | product/cli/db | db-commands | promoted_to_ac | db-ping and db-list-databases are CLI operato |
| 98 | system/db/connectivity | db-ping verifies the Dolt server is reachable and repor | PRODUCT | product/cli/db | db-commands | promoted_to_ac | db-ping and db-list-databases are CLI operato |
| 99 | system/db/connectivity | db-ping validates catalog readiness from the manifest b | PRODUCT | product/cli/db | db-commands | promoted_to_ac | db-ping and db-list-databases are CLI operato |
| 100 | system/db/connectivity | db-ping returns success when the server is reachable | PRODUCT | product/cli/db | db-commands | promoted_to_ac | db-ping and db-list-databases are CLI operato |
| 101 | system/operator-tooling | doctor reports the configured workflow backend, canonic | PRODUCT | product/cli/doctor | diagnostics | promoted_to_ac | Doctor is a CLI operator tool |
| 102 | system/operator-tooling | doctor surfaces whether COHERENCE_ENV is valid or inval | PRODUCT | product/cli/doctor | diagnostics | promoted_to_ac | Doctor is a CLI operator tool |
| 103 | system/operator-tooling | doctor reports what the effective catalog would be give | PRODUCT | product/cli/doctor | diagnostics | promoted_to_ac | Doctor is a CLI operator tool |
| 104 | system/operator-tooling | doctor reports whether the manifest has sufficient bind | PRODUCT | product/cli/doctor | diagnostics | promoted_to_ac | Doctor is a CLI operator tool |
| 105 | system/operator-tooling | doctor surfaces whether an explicit DOLT_DB environment | PRODUCT | product/cli/doctor | diagnostics | promoted_to_ac | Doctor is a CLI operator tool |
| 106 | system/operator-tooling | doctor locates the git work tree and reports the manife | PRODUCT | product/cli/doctor | diagnostics | promoted_to_ac | Doctor is a CLI operator tool |
| 107 | system/test-infrastructure | DoltWorld provides a disposable local Dolt instance for | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 108 | system/test-infrastructure | DoltWorld gracefully reports when the dolt CLI is unava | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 109 | system/test-infrastructure | DoltWorld can execute SQL queries against the managed D | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 110 | system/test-infrastructure | DoltWorld can start a temporary dolt sql-server for int | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 111 | system/test-infrastructure | DoltWorld temp directories are cleaned up when the worl | SYSTEM | system/test/world | test-primitives | evidence_only | Test/Infra surface - test infrastructure, not |
| 112 | system/operator-tooling | operator can drop a disposable coherence_test_* databas | PRODUCT | product/cli/db | drop-db | promoted_to_ac | Drop DB is a CLI operator command |
| 113 | system/operator-tooling | drop-isolated-test-db is a no-op when the project is in | PRODUCT | product/cli/db | drop-db | promoted_to_ac | Drop DB is a CLI operator command |
| 114 | system/operator-tooling | drop-isolated-test-db refuses to drop non-disposable da | PRODUCT | product/cli/db | drop-db | promoted_to_ac | Drop DB is a CLI operator command |
| 115 | system/evidence-persistence | evidence-sample creates the full directory layout: run. | PRODUCT | product/cli/evidence-sample | evidence-sample | promoted_to_ac | evidence-sample is a CLI operator command |
| 116 | system/evidence-persistence | canonical pointer stub is small metadata while actual a | FOUNDATION | foundation/infra/filesystem/evidence-layout | evidence-layout | promoted_to_ac | Evidence layout is infrastructure |
| 117 | system/evidence-persistence | evidence-sample demonstrates the ADR-0005 evidence layo | PRODUCT | product/cli/evidence-sample | evidence-sample | promoted_to_ac | evidence-sample is a CLI operator command |
| 118 | system/evidence-persistence | evidence-sample can target a specific workspace and run | PRODUCT | product/cli/evidence-sample | evidence-sample | promoted_to_ac | evidence-sample is a CLI operator command |
| 119 | system/evidence-persistence | evidence-sample auto-generates a unique run-id when not | PRODUCT | product/cli/evidence-sample | evidence-sample | promoted_to_ac | evidence-sample is a CLI operator command |
| 120 | system/evidence-persistence | evidence is stored under a deterministic per-run direct | FOUNDATION | foundation/infra/filesystem/evidence-layout | evidence-layout | promoted_to_ac | Evidence layout is infrastructure |
| 121 | system/evidence-persistence | evidence directories are created on demand with all par | FOUNDATION | foundation/infra/filesystem/evidence-layout | evidence-layout | promoted_to_ac | Evidence layout is infrastructure |
| 122 | system/evidence-persistence | each evidence run has a machine-readable manifest file  | FOUNDATION | foundation/infra/filesystem/evidence-layout | evidence-layout | promoted_to_ac | Evidence layout is infrastructure |
| 123 | system/evidence-persistence | large verification outputs are stored as artifact files | FOUNDATION | foundation/infra/filesystem/evidence-layout | evidence-layout | promoted_to_ac | Evidence layout is infrastructure |
| 124 | system/evidence-persistence | each verification observation produces a structured Sna | FOUNDATION | foundation/infra/filesystem/evidence-layout | evidence-layout | promoted_to_ac | Evidence layout is infrastructure |
| 125 | system/evidence-persistence | envelope files can be read back by observation id | FOUNDATION | foundation/infra/filesystem/evidence-layout | evidence-layout | promoted_to_ac | Evidence layout is infrastructure |
| 126 | system/evidence-persistence | evidence runs produce a future-compatible canonical poi | FOUNDATION | foundation/infra/filesystem/evidence-layout | evidence-layout | promoted_to_ac | Evidence layout is infrastructure |
| 127 | system/evidence-persistence | verification observations are represented as SnapshotEn | FOUNDATION | foundation/infra/filesystem/evidence-layout | evidence-layout | promoted_to_ac | Evidence layout is infrastructure |
| 128 | system/operator-tooling | the isolated test wrapper prints a preservation marker  | SYSTEM | system/process/test-isolation | test-world | evidence_only | Preservation marker is test-infra evidence |
| 129 | system/db/catalog-bootstrap | m0-smoke is the minimal smoke test for the full Rust-to | SYSTEM | system/process/test-isolation | smoke-test | evidence_only | Smoke test evidence - test infrastructure, no |
| 130 | system/db/catalog-bootstrap | m0-smoke refuses to run against the canonical catalog | SYSTEM | system/process/test-isolation | smoke-test | evidence_only | Smoke test evidence - test infrastructure, no |
| 131 | product/spec-authoring | m1-spec-smoke verifies the full spec/AC/relation CRUD s | SYSTEM | system/process/test-isolation | smoke-test | evidence_only | Smoke test evidence |
| 132 | product/spec-authoring | m1-spec-smoke refuses to run against the canonical cata | SYSTEM | system/process/test-isolation | smoke-test | evidence_only | Smoke test evidence |
| 133 | product/spec-authoring | m1-spec-smoke reports aggregate counts after smoke writ | SYSTEM | system/process/test-isolation | smoke-test | evidence_only | Smoke test evidence |
| 134 | system/db/catalog-bootstrap | migrate applies all pending embedded Refinery migration | FOUNDATION | foundation/infra/dolt/migrations | migration-contract | promoted_to_ac | Migrations are infrastructure provisioning |
| 135 | system/db/catalog-bootstrap | migrate validates catalog readiness from the manifest b | FOUNDATION | foundation/infra/dolt/migrations | migration-contract | promoted_to_ac | Migrations are infrastructure provisioning |
| 136 | system/db/catalog-bootstrap | migrate reports the target catalog and how many migrati | FOUNDATION | foundation/infra/dolt/migrations | migration-contract | promoted_to_ac | Migrations are infrastructure provisioning |
| 137 | system/db/catalog-bootstrap | migrate returns a success exit code when all migrations | FOUNDATION | foundation/infra/dolt/migrations | migration-contract | promoted_to_ac | Migrations are infrastructure provisioning |
| 138 | system/db/catalog-bootstrap | migrate provisions the database and applies all embedde | FOUNDATION | foundation/infra/dolt/migrations | migration-contract | promoted_to_ac | Migrations are infrastructure provisioning |
| 139 | system/db/catalog-bootstrap | spec and codeintel modules evolve their schemas indepen | FOUNDATION | foundation/infra/dolt/migrations | migration-contract | promoted_to_ac | Migrations are infrastructure provisioning |
| 140 | system/db/catalog-bootstrap | migrations use the same socket-first connection as CLI  | FOUNDATION | foundation/infra/dolt/migrations | migration-contract | promoted_to_ac | Migrations are infrastructure provisioning |
| 141 | system/db/catalog-bootstrap | migrate reports the combined count of applied migration | FOUNDATION | foundation/infra/dolt/migrations | migration-contract | promoted_to_ac | Migrations are infrastructure provisioning |
| 142 | system/spec-model | specs carry a level from a three-value enumeration: pro | FOUNDATION | foundation/domain/model/specs | model-invariant | promoted_to_ac | Domain model invariants belong to foundation |
| 143 | system/spec-model | specs carry a lifecycle status from a four-value enumer | FOUNDATION | foundation/domain/model/specs | model-invariant | promoted_to_ac | Domain model invariants belong to foundation |
| 144 | system/ac-model | ACs carry a review mode from a three-value enumeration | FOUNDATION | foundation/domain/model/acceptance-criteria | model-invariant | promoted_to_ac | AC model invariants belong to foundation |
| 145 | system/ac-model | ACs carry a risk level from a four-value enumeration | FOUNDATION | foundation/domain/model/acceptance-criteria | model-invariant | promoted_to_ac | AC model invariants belong to foundation |
| 146 | system/ac-model | ACs carry zero or more concern kinds from a five-value  | FOUNDATION | foundation/domain/model/acceptance-criteria | model-invariant | promoted_to_ac | AC model invariants belong to foundation |
| 147 | system/codeintel-model | codeintel code locations have a kind distinguishing tes | FOUNDATION | foundation/domain/model/codeintel-code-locations | codeintel-location-model | promoted_to_ac | Code location model is a separate domain conc |
| 148 | system/codeintel-model | AC-to-codeintel links have a relation kind from a three | FOUNDATION | foundation/domain/model/codeintel-ac-links | codeintel-link-model | promoted_to_ac | Codeintel links are a separate domain concept |
| 149 | system/spec-model | new specs have default slug derived from id, level defa | FOUNDATION | foundation/domain/model/specs | model-invariant | promoted_to_ac | Domain model invariants belong to foundation |
| 150 | system/ac-model | new ACs have defaults: slug from id, manual review mode | FOUNDATION | foundation/domain/model/acceptance-criteria | model-invariant | promoted_to_ac | AC model invariants belong to foundation |
| 151 | system/spec-model | identifiers are normalized to lowercase hyphenated slug | FOUNDATION | foundation/domain/model/specs | model-invariant | promoted_to_ac | Domain model invariants belong to foundation |
| 152 | system/spec-model | the full spec graph can be loaded as a bulk snapshot co | FOUNDATION | foundation/domain/model/specs | model-invariant | promoted_to_ac | Domain model invariants belong to foundation |
| 153 | system/codeintel-model | codeintel links can be retrieved with their associated  | FOUNDATION | foundation/domain/model/codeintel-ac-links | codeintel-link-model | promoted_to_ac | Codeintel links are a separate domain concept |
| 154 | system/operator-tooling | operator can verify that the project has a valid manife | PRODUCT | product/cli/project | project-init-reset | promoted_to_ac | Project init/reset is a CLI operator command |
| 155 | system/operator-tooling | project init binds the git worktree identity once by wr | PRODUCT | product/cli/project | project-init-reset | promoted_to_ac | Project init/reset is a CLI operator command |
| 156 | system/operator-tooling | operator can bootstrap a new project manifest with a sl | PRODUCT | product/cli/project | project-init-reset | promoted_to_ac | Project init/reset is a CLI operator command |
| 157 | system/operator-tooling | project init does not overwrite an already-bound projec | PRODUCT | product/cli/project | project-init-reset | promoted_to_ac | Project init/reset is a CLI operator command |
| 158 | system/operator-tooling | operator can force a new bind to recover from orphaned  | PRODUCT | product/cli/project | project-init-reset | promoted_to_ac | Project init/reset is a CLI operator command |
| 159 | system/operator-tooling | project init can be run from outside the git work tree  | PRODUCT | product/cli/project | project-init-reset | promoted_to_ac | Project init/reset is a CLI operator command |
| 160 | system/operator-tooling | project init requires git access to resolve the worktre | PRODUCT | product/cli/project | project-init-reset | promoted_to_ac | Project init/reset is a CLI operator command |
| 161 | system/operator-tooling | project reset provides an idempotent repair path that k | PRODUCT | product/cli/project | project-init-reset | promoted_to_ac | Project init/reset is a CLI operator command |
| 162 | system/operator-tooling | the git repository root can be discovered by walking pa | FOUNDATION | foundation/infra/filesystem/project-manifest | manifest-contract | promoted_to_ac | Project manifest is infrastructure |
| 163 | system/operator-tooling | the project manifest is a TOML file at .coherence/proje | FOUNDATION | foundation/infra/filesystem/project-manifest | manifest-contract | promoted_to_ac | Project manifest is infrastructure |
| 164 | system/operator-tooling | the project manifest can be written and persists to .co | FOUNDATION | foundation/infra/filesystem/project-manifest | manifest-contract | promoted_to_ac | Project manifest is infrastructure |
| 165 | system/operator-tooling | project_hash binding requires manifest schema version 2 | FOUNDATION | foundation/infra/filesystem/project-manifest | manifest-contract | promoted_to_ac | Project manifest is infrastructure |
| 166 | system/db/connectivity | arbitrary project identifiers are sanitized to MySQL-sa | FOUNDATION | foundation/infra/dolt/catalog-naming | catalog-naming | promoted_to_ac | Catalog naming is an infra/connectivity conce |
| 167 | system/db/connectivity | git toplevel paths are reduced to a 4-character hex has | FOUNDATION | foundation/infra/dolt/catalog-naming | catalog-naming | promoted_to_ac | Catalog naming is an infra/connectivity conce |
| 168 | system/db/connectivity | legacy catalog binding composes a slug and git-path has | FOUNDATION | foundation/infra/dolt/catalog-naming | catalog-naming | promoted_to_ac | Catalog naming is an infra/connectivity conce |
| 169 | system/db/connectivity | normalized catalog name is slug_hash_env with truncatio | FOUNDATION | foundation/infra/dolt/catalog-naming | catalog-naming | promoted_to_ac | Catalog naming is an infra/connectivity conce |
| 170 | system/db/connectivity | COHERENCE_ENV parsing is case-insensitive and defaults  | FOUNDATION | foundation/infra/dolt/catalog-naming | catalog-naming | promoted_to_ac | Catalog naming is an infra/connectivity conce |
| 171 | system/db/connectivity | COHERENCE_ENV environment variable controls the logical | FOUNDATION | foundation/infra/dolt/catalog-naming | catalog-naming | promoted_to_ac | Catalog naming is an infra/connectivity conce |
| 172 | system/operator-tooling | manifest reading is best-effort from cwd and returns No | FOUNDATION | foundation/infra/filesystem/project-manifest | manifest-contract | promoted_to_ac | Project manifest is infrastructure |
| 173 | system/cli/spec-command | spec command requires an explicit add/list/show subcomm | SYSTEM | system/process/spec-authoring | subcommand-dispatch | promoted_to_ac | CLI subcommand dispatch belongs to spec-autho |
| 174 | system/cli/spec-command | spec command rejects unknown subcommands with expected  | SYSTEM | system/process/spec-authoring | subcommand-dispatch | promoted_to_ac | CLI subcommand dispatch belongs to spec-autho |
| 175 | product/spec-authoring | operator must provide a stable spec slug when authoring | PRODUCT | product/cli/spec | spec-add | promoted_to_ac | Spec authoring UX is product/cli/spec |
| 176 | product/spec-authoring | operator must provide a human title when authoring a sp | PRODUCT | product/cli/spec | spec-add | promoted_to_ac | Spec authoring UX is product/cli/spec |
| 177 | product/spec-authoring | operator may omit description when creating a draft spe | PRODUCT | product/cli/spec | spec-add | promoted_to_ac | Spec authoring UX is product/cli/spec |
| 178 | system/spec-model | new specs have a level and default to module when omitt | FOUNDATION | foundation/domain/model/specs | model-defaults | promoted_to_ac | Model defaults are foundation/domain behavior |
| 179 | system/spec-model | new specs start as draft unless a valid status is suppl | FOUNDATION | foundation/domain/model/specs | model-defaults | promoted_to_ac | Model defaults are foundation/domain behavior |
| 180 | system/spec-identity | CLI can generate a spec id when the operator does not p | FOUNDATION | foundation/domain/model/specs | id-generation | moved_to_lower_level | ID generation is part of spec model lifecycle |
| 181 | system/db/catalog-bootstrap | spec commands run against a migrated catalog schema | FOUNDATION | foundation/infra/dolt/migrations | migration-contract | promoted_to_ac | Migrations are infrastructure provisioning |
| 182 | product/spec-authoring | successful spec creation returns the created identity t | PRODUCT | product/cli/spec | spec-add | promoted_to_ac | Spec authoring UX is product/cli/spec |
| 183 | product/spec-browsing | spec list has no positional or flag arguments in curren | PRODUCT | product/cli/spec | spec-list-show | promoted_to_ac | Spec browsing is part of product/cli/spec |
| 184 | product/spec-browsing | operator can list specs with identity and lifecycle fie | PRODUCT | product/cli/spec | spec-list-show | promoted_to_ac | Spec browsing is part of product/cli/spec |
| 185 | product/spec-browsing | operator can inspect a spec by id through positional or | PRODUCT | product/cli/spec | spec-list-show | promoted_to_ac | Spec browsing is part of product/cli/spec |
| 186 | product/spec-browsing | spec inspection fails clearly for unknown ids | PRODUCT | product/cli/spec | spec-list-show | promoted_to_ac | Spec browsing is part of product/cli/spec |
| 187 | product/spec-browsing | operator can inspect persisted title, description, leve | PRODUCT | product/cli/spec | spec-list-show | promoted_to_ac | Spec browsing is part of product/cli/spec |
| 188 | system/spec-model | specs can be created or updated via upsert; all mutable | COMPONENT | component/repository/spec-store | persistence-contract | promoted_to_ac | Repository persistence contract |
| 189 | system/spec-model | specs can be retrieved individually by id | COMPONENT | component/repository/spec-store | persistence-contract | promoted_to_ac | Repository persistence contract |
| 190 | system/spec-model | all specs can be listed in id order | COMPONENT | component/repository/spec-store | persistence-contract | promoted_to_ac | Repository persistence contract |
| 191 | system/spec-model | a full spec graph can be loaded in bulk for the TUI and | COMPONENT | component/repository/spec-store | persistence-contract | promoted_to_ac | Repository persistence contract |
| 192 | system/ac-model | ACs can be created or updated via upsert; concerns are  | FOUNDATION | foundation/domain/model/acceptance-criteria | model-invariant | promoted_to_ac | AC model invariants belong to foundation |
| 193 | system/ac-model | AC slug must not be empty when persisted | FOUNDATION | foundation/domain/model/acceptance-criteria | model-invariant | promoted_to_ac | AC model invariants belong to foundation |
| 194 | system/ac-model | each AC has a unique slug within its parent spec | FOUNDATION | foundation/domain/model/acceptance-criteria | model-invariant | promoted_to_ac | AC model invariants belong to foundation |
| 195 | system/ac-model | ACs can be retrieved individually by id with their conc | FOUNDATION | foundation/domain/model/acceptance-criteria | model-invariant | promoted_to_ac | AC model invariants belong to foundation |
| 196 | system/ac-model | all ACs for a given spec can be listed | FOUNDATION | foundation/domain/model/acceptance-criteria | model-invariant | promoted_to_ac | AC model invariants belong to foundation |
| 197 | system/spec-model | spec relations can be created or updated via upsert | COMPONENT | component/repository/spec-store | persistence-contract | promoted_to_ac | Repository persistence contract |
| 198 | system/spec-model | spec relations for a given spec can be listed in both d | COMPONENT | component/repository/spec-store | persistence-contract | promoted_to_ac | Repository persistence contract |
| 199 | system/operator-tooling | mutating smoke and tests are refused unless COHERENCE_D | SYSTEM | system/process/test-isolation | isolation-policy | promoted_to_ac | Test isolation is a system process concern |
| 200 | system/operator-tooling | the isolation guard permits writes when the test profil | SYSTEM | system/process/test-isolation | isolation-policy | promoted_to_ac | Test isolation is a system process concern |
| 201 | system/operator-tooling | the isolation guard refuses writes when COHERENCE_DB_PR | SYSTEM | system/process/test-isolation | isolation-policy | promoted_to_ac | Test isolation is a system process concern |
| 202 | system/operator-tooling | writes to a database matching the canonical project slu | SYSTEM | system/process/test-isolation | isolation-policy | promoted_to_ac | Test isolation is a system process concern |
| 203 | system/operator-tooling | databases prefixed with coherence_test_ are always perm | SYSTEM | system/process/test-isolation | isolation-policy | promoted_to_ac | Test isolation is a system process concern |
| 204 | system/operator-tooling | explicitly allowlisted database names bypass the dispos | SYSTEM | system/process/test-isolation | isolation-policy | promoted_to_ac | Test isolation is a system process concern |
| 205 | system/operator-tooling | isolated test workflows cannot accidentally target the  | SYSTEM | system/process/test-isolation | isolation-policy | promoted_to_ac | Test isolation is a system process concern |
| 206 | system/operator-tooling | COHERENCE_TEST_WORLD env var can assert the expected da | SYSTEM | system/process/test-isolation | isolation-policy | promoted_to_ac | Test isolation is a system process concern |
| 207 | product/tui-navigation | the TUI tree presents specs grouped by level with colla | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 208 | product/tui-navigation | clicking or pressing enter on a tree node expands it to | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 209 | product/tui-navigation | selecting a tree item updates the preview panel with th | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 210 | product/tui-verification | the TUI executes a defined set of effects for spec/AC p | PRODUCT | product/tui/editing | effects | promoted_to_ac | TUI effects are part of editing surface |
| 211 | product/tui-navigation | editing a spec and confirming persists the change and u | PRODUCT | product/tui/editing | draft-edit | promoted_to_ac | Draft editing is a separate product surface |
| 212 | product/tui-navigation | editing an AC and confirming persists the change and up | PRODUCT | product/tui/editing | draft-edit | promoted_to_ac | Draft editing is a separate product surface |
| 213 | product/tui-navigation | operator can edit a spec description in their preferred | PRODUCT | product/tui/editing | draft-edit | promoted_to_ac | Draft editing is a separate product surface |
| 214 | product/tui-navigation | operator can edit an AC intent in their preferred edito | PRODUCT | product/tui/editing | draft-edit | promoted_to_ac | Draft editing is a separate product surface |
| 215 | product/tui-navigation | the TUI can reload the spec graph from the database, sw | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 216 | product/tui-verification | the TUI can verify a single AC and display the result i | PRODUCT | product/tui/verification | tui-verification | promoted_to_ac | TUI verification is product/tui/verification |
| 217 | product/tui-verification | the TUI can verify all specs in the tree and update all | PRODUCT | product/tui/verification | tui-verification | promoted_to_ac | TUI verification is product/tui/verification |
| 218 | product/tui-navigation | the TUI keyboard navigation follows a defined key bindi | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 219 | product/tui-verification | verification keyboard shortcuts are only active when br | PRODUCT | product/tui/verification | tui-verification | promoted_to_ac | TUI verification is product/tui/verification |
| 220 | product/tui-navigation | editing a spec or AC in the TUI uses single-key shortcu | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 221 | product/tui-navigation | the TUI has three navigable screens with stateful tree, | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 222 | product/tui-navigation | the TUI tree viewport scrolls to keep the selected item | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 223 | product/tui-navigation | selecting a tree item updates the detail panel with the | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 224 | product/tui-navigation | the TUI offers dev, test, and prod environment selectio | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 225 | product/tui-navigation | the tui command launches the TUI binary | PRODUCT | product/cli/tui | tui-launch | promoted_to_ac | TUI launch is CLI entry point |
| 226 | product/tui-navigation | the TUI edit draft tracks both original and pending val | PRODUCT | product/tui/editing | draft-edit | promoted_to_ac | Draft editing is a separate product surface |
| 227 | product/tui-navigation | the TUI can detect whether unsaved edits exist in the d | PRODUCT | product/tui/editing | draft-edit | promoted_to_ac | Draft editing is a separate product surface |
| 228 | product/tui-navigation | the TUI validates draft fields before persisting | PRODUCT | product/tui/editing | draft-edit | promoted_to_ac | Draft editing is a separate product surface |
| 229 | product/tui-navigation | the TUI auto-discovers coherence projects under $HOME/g | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 230 | product/tui-navigation | the TUI connects to the Dolt database with a connection | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 231 | product/tui-navigation | the TUI repository surface provides a complete spec/AC/ | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 232 | product/tui-navigation | the TUI renders with a defined color theme applied cons | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 233 | product/tui-navigation | the TUI tree renders only visible items with correct vi | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 234 | product/tui-navigation | tree items show expand/collapse markers, indentation, a | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 235 | product/tui-navigation | the detail panel shows spec and AC fields using a box-d | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 236 | product/tui-navigation | the project and environment pickers render as themed li | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 237 | product/tui-navigation | the TUI uses a three-row layout: title bar, main area,  | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 238 | product/tui-navigation | the TUI event loop processes actions by updating state  | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 239 | product/tui-navigation | the TUI progresses through three screens in order: proj | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 240 | product/tui-navigation | the back action reverses through the screen stack and c | PRODUCT | product/tui/navigation | tree-navigation | promoted_to_ac | TUI navigation is product/tui/navigation |
| 241 | product/verification | operator can verify an AC by id via positional argument | PRODUCT | product/cli/verify | verify-cli | promoted_to_ac | Verification CLI is product/cli/verify |
| 242 | product/verification | verify-ac exits with status reflecting the outcome of a | PRODUCT | product/cli/verify | verify-cli | promoted_to_ac | Verification CLI is product/cli/verify |
| 243 | product/verification | verify-ac reports aggregate outcome per AC | PRODUCT | product/cli/verify | verify-cli | promoted_to_ac | Verification CLI is product/cli/verify |
| 244 | product/verification | verification reports clearly when an AC has no executab | PRODUCT | product/cli/verify | verify-cli | promoted_to_ac | Verification CLI is product/cli/verify |
| 245 | product/verification | each linked verification command produces a LINK row wi | PRODUCT | product/cli/verify | verify-cli | promoted_to_ac | Verification CLI is product/cli/verify |
| 246 | system/evidence-persistence | verification runs with a run-id persist per-link observ | SYSTEM | system/process/evidence-capture | evidence-persistence | promoted_to_ac | Evidence persistence during verification is a |
| 247 | system/evidence-persistence | evidence write failures are best-effort and never fail  | SYSTEM | system/process/evidence-capture | evidence-persistence | promoted_to_ac | Evidence persistence during verification is a |
| 248 | product/verification | operator can verify all ACs under a spec via positional | PRODUCT | product/cli/verify | verify-cli | promoted_to_ac | Verification CLI is product/cli/verify |
| 249 | product/verification | verify-spec exits with aggregate status reflecting all  | PRODUCT | product/cli/verify | verify-cli | promoted_to_ac | Verification CLI is product/cli/verify |
| 250 | product/verification | verify-spec reports aggregate counts for the entire spe | PRODUCT | product/cli/verify | verify-cli | promoted_to_ac | Verification CLI is product/cli/verify |
| 251 | product/verification | verify-spec surfaces per-AC outcome within the spec | PRODUCT | product/cli/verify | verify-cli | promoted_to_ac | Verification CLI is product/cli/verify |
| 252 | product/verification | verification output structure is consistent whether inv | PRODUCT | product/cli/verify | verify-cli | promoted_to_ac | Verification CLI is product/cli/verify |
