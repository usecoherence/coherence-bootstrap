//! Isolated test-world guard (ADR-0004): refuse smoke and integration-test writes without an explicit profile.
//! The canonical repository Dolt catalog holds **curated reasoning state**; automated tests and mutating smoke
//! must use a disposable target with `COHERENCE_DB_PROFILE=test`.

use std::env;

/// Environment variable that must name an isolated profile before mutating workflows run.
pub const PROFILE_ENV_VAR: &str = "COHERENCE_DB_PROFILE";

/// Required value for [`PROFILE_ENV_VAR`] (ASCII case-insensitive) before smoke or tests issue writes
/// such as migrations, fixtures, or codeintel linkage on the configured Dolt target.
pub const ISOLATED_PROFILE_VALUE: &str = "test";

/// Shared footer for refusal messages so CLI, scripts, and docs stay aligned on what to run next.
pub const STANDARD_REMEDIATION_TARGETS: &str =
    "Next targets (repository root):\n\
     • Workspace tests — `make test-isolated` or `make tool run`\n\
     • Mutating smoke (writes fixtures) — `make smoke`\n\
     • Optional teardown of disposable `.dolt` — `make tool dolt-stop` then `make test-world-reset` (skip reset after a failed run if you need to inspect state)\n";

fn profile_allows_writes(raw: &str) -> bool {
    raw.trim().eq_ignore_ascii_case(ISOLATED_PROFILE_VALUE)
}

fn profile_cause_summary(raw: &str) -> String {
    if raw.trim().is_empty() {
        format!(
            "unset — canonical/default session; not an isolated test world (need `{PROFILE_ENV_VAR}={ISOLATED_PROFILE_VALUE}`)"
        )
    } else {
        format!("{raw:?} — non-test profile (need `{PROFILE_ENV_VAR}={ISOLATED_PROFILE_VALUE}`)")
    }
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

    let cause = profile_cause_summary(&raw);
    Err(format!(
        "{context}: blocked: refusing database writes for smoke/tests — isolated test-world profile required (ADR-0004).\n\
         \n\
         Cause: `{PROFILE_ENV_VAR}` is {cause}.\n\
         \n\
         {STANDARD_REMEDIATION_TARGETS}\
         \n\
         Fix: `export {PROFILE_ENV_VAR}={ISOLATED_PROFILE_VALUE}` and point `DOLT_DB` / `DOLT_SOCKET` (or TCP `DOLT_HOST`/`DOLT_PORT`) at a **disposable** Dolt database, not the curated canonical checkout catalog.",
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn refusal_includes_stable_next_targets_when_unset() {
        let _g = ENV_LOCK.lock().unwrap();
        env::remove_var(PROFILE_ENV_VAR);
        let msg = require_isolated_test_world_for_writes("test_ctx").unwrap_err();
        assert!(
            msg.contains("ADR-0004"),
            "expected ADR marker in refusal: {msg}"
        );
        assert!(
            msg.contains("make test-isolated"),
            "expected test-isolated hint: {msg}"
        );
        assert!(msg.contains("make smoke"), "expected smoke hint: {msg}");
        assert!(
            msg.contains("test-world-reset"),
            "expected teardown hint: {msg}"
        );
    }

    #[test]
    fn ok_when_profile_test() {
        let _g = ENV_LOCK.lock().unwrap();
        env::set_var(PROFILE_ENV_VAR, "test");
        require_isolated_test_world_for_writes("test_ctx").unwrap();
        env::remove_var(PROFILE_ENV_VAR);
    }

    #[test]
    fn refusal_when_profile_non_test() {
        let _g = ENV_LOCK.lock().unwrap();
        env::set_var(PROFILE_ENV_VAR, "canonical");
        let msg = require_isolated_test_world_for_writes("test_ctx").unwrap_err();
        assert!(msg.contains("non-test profile"), "unexpected: {msg}");
        env::remove_var(PROFILE_ENV_VAR);
    }
}
