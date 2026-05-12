# Technical: command router (one layer)

**Classification:** observed — maps to `cli::run` in `src/cli.rs`.

## Flow

1. Read `args[1]` as command name; if missing, use `"help"`.
2. `match` exact string tokens (no subcommand discovery at this layer).
3. Delegation:
   - `spec`, `ac`, `ac-tests`, `verify-ac`, `verify-spec`, `evidence-sample`, `project` → pass `&args[2..]` into `commands::<module>::run`.
   - Zero-arg commands (`doctor`, `migrate`, …) → `run()` with no slice.
4. Return integer exit code to `main` → `std::process::exit`.

## Semantic roles

| Router concern | Implementation |
|----------------|----------------|
| Aliasing | `help` / `--help` / `-h`; `version` / `--version` / `-V`. |
| Unknown | Fallback arm: stderr + exit **64**. |
| Subcommand errors | Mostly **1** inside `commands::*` (per-module `eprintln!` + return). |

**Catalog wording:** AC intents imported into the catalog for router behavior should stay **operator/behavioral**; file-level routing detail lives in this technical note (`cli.rs` → `commands/*`).

## Not done at router layer

- No global option parsing (e.g. no `--verbose` at top level).
- No automatic `migrate` before arbitrary commands (each command module decides whether to call `migrations::apply_all` / `connect_migrated`).
