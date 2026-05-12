# Commands: smoke / debug — `m0-smoke`, `m1-spec-smoke`

**Classification:** observed / **internal candidate** — useful for CI and vertical-slice proofs; not the primary operator-facing product map unless explicitly promoted.

## `m0-smoke`

**Public intent (as implemented):** Minimal “Rust → migrated Dolt → insert spec + AC via low-level db helpers → read counts” smoke.

**World:** **`require_isolated_test_world_for_writes`** — must be disposable / test-profile catalog; applies migrations, connects, inserts fixed ids `SPEC-1` / `AC-1`, prints counts.

**Observable:** stdout progress lines; mutates configured test database.

**Exit:** **0** ok, **1** with `m0-smoke:` errors.

---

## `m1-spec-smoke`

**Public intent (as implemented):** Exercise **spec_store** APIs for spec + AC + `spec_relations` with stable `M1-SMOKE-*` ids; list/count sanity.

**World:** same isolated guard as m0; migrations + connect + store operations.

**Exit:** **0** / **1** pattern.

---

## Reverse-spec stance

- Treat as **diagnostic / harness-adjacent** commands when explaining the product.
- When capturing **observed** behavior for catalog import later, either:
  - file under a SPEC like “workspace smoke harness”, or
  - exclude from initial operator SPEC set.

Do not conflate smoke success with “production catalog is healthy” without extra checks.
