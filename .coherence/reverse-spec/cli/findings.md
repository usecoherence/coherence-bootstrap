# CLI reverse-spec — findings (living)

**Classification:** mix of observed quirks and **accidental / internal** candidates — not commitments until promoted to SPEC/AC in catalog.

## Router / UX

- **Unknown command exit 64** vs most command failures exit **1** — dual convention; document clearly for operators and future CLI consistency work.
- **Default argv** maps to `help`, not `doctor` or `migrate`.

## `doctor`

- Returns **0** even when only printing “invalid `COHERENCE_ENV`” style guidance — “doctor always ok” may surprise operators expecting non-zero on bad env (accidental candidate vs desired strict doctor).

## `version`

- Version string is **hardcoded** in `cli.rs` (`0.1.0`) — may drift from `Cargo.toml`; observed mismatch risk.

## `spec` / `ac`

- No `delete` / `update` subcommands in router — add-only + list/show surface (observed capability boundary).

## `verify-ac` / `verify-spec`

- **No `verified_by` links → exit 0** — convenient for incremental rollout; risky if mistaken for “AC verified” (see main draft: accidental candidate).

## `drop-isolated-test-db`

- **Exit 0 when “skipped”** if user-scoped Dolt off — distinguish from “dropped successfully” in operator docs later.

## Future work (do not implement in reverse-spec pass)

- Black-box **world harness** (container/Dagger) should own sockets/DB/git — not hand-written SQL as the primary artifact (per revised plan).
- Optional: generated **technical/** deep dives per command file once command-group SPECs exist in catalog.
