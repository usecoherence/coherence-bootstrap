# Reverse-spec: black-box world harness

**Scope (PR-3):** design + **L0** shell checks only — no Dolt, no `crates/**` edits. Everything here is **reverse-spec tooling** under `.coherence/reverse-spec/world/`.

## Four entities

| Entity | What it is |
|--------|------------|
| **SUT / oracle** | The shipped `coherence-core-db` binary from this repo. **Immutable** source tree for reverse-spec. |
| **Control catalog** | Dolt DB holding reverse-spec SPEC/AC/`verified_by` rows (seeded e.g. via `pr2-catalog-import/`). **`verify-ac` reads this** to find what to run. |
| **Test world** | Disposable DB, env, temp dirs where **inner** CLI invocations mutate state. Inner commands must use `COHERENCE_DB_PROFILE=test` and a **throwaway `DOLT_DB`**, not the control catalog. |
| **Evidence** | Captured `stdout` / `stderr` / exit code / optional DB snapshots under `world/evidence/<run-id>/<ac-or-step>/`. |

**Key rule (later, when `verify-ac` drives checks):** outer `verify-ac` talks to **control**; linked `test_command` should call a **world runner** (e.g. `world/bin/run-ac …`) that provisions **test** world, runs inner CLI, asserts, writes evidence — not a giant inline shell string in SQL.

## Isolation levels (L0–L4)

| Level | Needs | Examples |
|-------|--------|------------|
| **L0** | Binary only (no Dolt) | `help`, `version`, unknown command |
| **L1** | Dolt + migrated disposable DB | `spec`, `ac`, `migrate`, `db-ping` |
| **L2** | Git + `.coherence/project.toml` paths | `project`, parts of `doctor` narrative |
| **L3** | Shell / filesystem / evidence dirs | `verify-ac`, `verify-spec`, `evidence-sample`, `ac-tests` |
| **L4** | Full fresh container, non-root, no writable host mounts | Anything hostile or unknown blast radius |

**Start simple:** this PR implements **L0** scripts only. Later: disposable DB per AC (L1+), then container-per-run (L4) for safety.

## Rollback vs fresh world

Prefer **fresh disposable `DOLT_DB` per AC** (or per test) over in-process SQL transactions: the CLI can spawn subprocesses, write files, touch `.coherence/`. Transactions do not bound whole runtime.

## Container / Dagger (later)

Reproduce **dev/prod-ish** layout inside a container: copy repo to `/work/repo`, start Dolt in-container, recreate control catalog from **git-tracked** import scripts (not mystery host DB state), run `world-run` / `run-ac`, collect evidence, destroy container.

## Layout

```text
world/
  README.md          (this file)
  bin/
    world-run        orchestrates a batch (e.g. all L0)
    run-ac           dispatches one AC id → ac/<AC_ID>.sh
  ac/
    AC-CLI-SURFACE-HELP.sh
    AC-CLI-SURFACE-VERSION.sh
    AC-CLI-SURFACE-UNKNOWN.sh
  evidence/
    <run-id>/...     (gitignored artifacts; .gitkeep only in evidence/)
```

**Do not** mutate host checkout except under `world/evidence/` (and optional future `world/tmp/` inside same tree). Prefer running from CI with a clean working tree.

## `verified_by` shape (future)

Store short invocations:

```text
.coherence/reverse-spec/world/bin/run-ac AC-SOME-CHECK
```

The runner creates test DB, migrates, sets env, snapshots, runs inner `coherence-core-db …`, compares, writes evidence, exits 0/1.

`world-run` sets **`COHERENCE_WORLD_RUN_ID`** once per batch so all L0 checks share `evidence/<run-id>/…`.
