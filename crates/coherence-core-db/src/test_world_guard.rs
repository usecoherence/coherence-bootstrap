//! Isolated test-world guard (ADR-0004): refuse smoke and integration-test writes without an explicit profile.

use std::env;

/// Environment variable that must name an isolated profile before mutating workflows run.
pub const PROFILE_ENV_VAR: &str = "COHERENCE_DB_PROFILE";

/// Required value for [`PROFILE_ENV_VAR`] (ASCII case-insensitive) before smoke or tests issue writes
/// such as migrations, fixtures, or codeintel linkage on the configured Dolt target.
pub const ISOLATED_PROFILE_VALUE: &str = "test";

fn profile_allows_writes(raw: &str) -> bool {
    raw.trim().eq_ignore_ascii_case(ISOLATED_PROFILE_VALUE)
}

/// Returns `Ok(())` when this process may run mutating smoke or test workflows against `DOLT_*`.
///
/// `context` labels the caller in errors (for example `m0-smoke` or a test module path).
#[must_use = "caller must propagate or handle refusal"]
pub fn require_isolated_test_world_for_writes(context: &str) -> Result<(), String> {
    let raw = env::var(PROFILE_ENV_VAR).unwrap_or_default();
    if profile_allows_writes(&raw) {
        return Ok(());
    }

    let current_summary = if raw.trim().is_empty() {
        format!(
            "unset — treated as canonical/default; not an isolated test world (set `{PROFILE_ENV_VAR}={ISOLATED_PROFILE_VALUE}`)"
        )
    } else {
        format!(
            "{raw:?} — refusing non-test profiles for mutating smoke/tests (want `{PROFILE_ENV_VAR}={ISOLATED_PROFILE_VALUE}`)"
        )
    };

    Err(format!(
        "{context}: blocked: refusing to mutate the database without an isolated test-world profile.\n\
         \n\
         Cause: `{PROFILE_ENV_VAR}` is {current_summary}.\n\
         \n\
         Fix: export `{PROFILE_ENV_VAR}={ISOLATED_PROFILE_VALUE}` and point `DOLT_DB` / `DOLT_SOCKET` (or TCP `DOLT_HOST`/`DOLT_PORT`) at a **disposable** Dolt database, not curated canonical checkout data.\n\
         \n\
         Workspace `cargo test` is run under this profile via `scripts/run` and CI.\n\
         Smoke commands require the same when invoked manually.",
    ))
}

/// Same policy as [`require_isolated_test_world_for_writes`] but panic with the refusal text (crate unit tests).
#[cfg(test)]
#[track_caller]
pub fn panic_unless_isolated_test_world_for_writes(context: &str) {
    if let Err(msg) = require_isolated_test_world_for_writes(context) {
        panic!("{msg}");
    }
}
