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


