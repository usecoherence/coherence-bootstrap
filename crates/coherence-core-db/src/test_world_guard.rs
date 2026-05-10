//! Isolated test-world guard (ADR-0004): refuse smoke and integration-test writes unless
//! `COHERENCE_DB_PROFILE=test` and, on the **user-scoped shared Dolt** layout (ADR-0006), the resolved
//! [`ConnectionConfig`] points at a disposable or otherwise non-canonical database name (see [`require_isolated_test_world_for_writes`]).
//! Repo-local `.dolt` catalogs use the legacy **profile=test-only** gate so checkout workflows remain usable.

use std::env;

use crate::db::{user_scoped_dolt_from_env, ConnectionConfig};

/// Environment variable that must name an isolated profile before mutating workflows run.
pub const PROFILE_ENV_VAR: &str = "COHERENCE_DB_PROFILE";

/// Required value for [`PROFILE_ENV_VAR`] (ASCII case-insensitive) before smoke or tests issue writes
/// such as migrations, fixtures, or codeintel linkage on the configured Dolt target.
pub const ISOLATED_PROFILE_VALUE: &str = "test";

/// Canonical project slug env (ADR-0004): when set, writes are refused against a resolved [`ConnectionConfig`] whose
/// `database` matches this value (ASCII case-insensitive).
pub const PROJECT_SLUG_ENV_VAR: &str = "COHERENCE_PROJECT_SLUG";

/// Comma-separated list of resolved database names allowed under `profile=test` in addition to the disposable prefix rule.
pub const TEST_WORLD_ALLOWLIST_ENV_VAR: &str = "COHERENCE_TEST_WORLD_DB_ALLOWLIST";

/// Optional sanity check (design note): when non-empty, must match resolved `database` ASCII case-insensitively.
pub const TEST_WORLD_EXPECTED_DB_ENV_VAR: &str = "COHERENCE_TEST_WORLD";

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

fn configured_test_db_prefix() -> String {
    env::var("COHERENCE_TEST_DB_PREFIX").unwrap_or_else(|_| "coherence_test_".to_string())
}

fn parse_project_slug_opt() -> Option<String> {
    let raw = env::var(PROJECT_SLUG_ENV_VAR).unwrap_or_default();
    let t = raw.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn allowlisted_databases() -> Vec<String> {
    env::var(TEST_WORLD_ALLOWLIST_ENV_VAR)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn database_is_explicitly_allowlisted(database: &str) -> bool {
    allowlisted_databases()
        .iter()
        .any(|entry| database.eq_ignore_ascii_case(entry))
}

fn database_has_disposable_prefix(database: &str) -> bool {
    let prefix = configured_test_db_prefix();
    database.starts_with(&prefix)
}

/// Resolved identity checks (disposable naming, slug equality, allow-list) apply only under the
/// user-scoped shared Dolt layout (ADR-0006): `scripts/with-isolated-test-profile` provisions
/// disposable `DOLT_DB` names in that mode. Repo-local `.dolt` catalogs keep the historical
/// `COHERENCE_DB_PROFILE=test`-only gate so `make tool run` / `make test-isolated` work without extra env.
fn resolved_identity_checks_active() -> bool {
    user_scoped_dolt_from_env()
}

/// Under user-scoped Dolt, refuses resolved `database` names that cannot be proven disposable or non-canonical.
fn resolved_database_identity_allows_writes(config: &ConnectionConfig) -> Result<(), String> {
    if !resolved_identity_checks_active() {
        return Ok(());
    }

    let database = config.database.as_str();
    let slug_opt = parse_project_slug_opt();

    if let Some(ref slug) = slug_opt {
        if database.eq_ignore_ascii_case(slug) {
            return Err(format!(
                "resolved `DOLT_DB` matches canonical `{PROJECT_SLUG_ENV_VAR}`={slug:?} (ASCII case-insensitive); refusing mutating workflows against curated catalog identity"
            ));
        }
    }

    let disposable_ok = database_has_disposable_prefix(database);
    let allowlist_ok = database_is_explicitly_allowlisted(database);
    let non_canonical_named = slug_opt.is_some(); // ensured != slug above when slug set

    if disposable_ok || allowlist_ok || non_canonical_named {
        Ok(())
    } else {
        let prefix = configured_test_db_prefix();
        Err(format!(
            "resolved `DOLT_DB`={database:?} is not disposable (expected prefix {prefix:?} via `COHERENCE_TEST_DB_PREFIX` or an entry in `{TEST_WORLD_ALLOWLIST_ENV_VAR}`), and `{PROJECT_SLUG_ENV_VAR}` is unset — cannot prove a non-canonical disposable target",
        ))
    }
}

fn check_test_world_expectation(database: &str) -> Result<(), String> {
    let Ok(raw) = env::var(TEST_WORLD_EXPECTED_DB_ENV_VAR) else {
        return Ok(());
    };
    let expected = raw.trim();
    if expected.is_empty() {
        return Ok(());
    }
    if database.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!(
            "`{TEST_WORLD_EXPECTED_DB_ENV_VAR}` is set to {expected:?} but resolved `DOLT_DB`={database:?}"
        ))
    }
}

/// Returns `Ok(())` when this process may run mutating smoke or test workflows against the resolved [`ConnectionConfig`].
///
/// `context` labels the caller in errors (for example `m0-smoke` or a test module path).
#[must_use = "caller must propagate or handle refusal"]
pub fn require_isolated_test_world_for_writes(
    context: &str,
    config: &ConnectionConfig,
) -> Result<(), String> {
    let raw = env::var(PROFILE_ENV_VAR).unwrap_or_default();
    if !profile_allows_writes(&raw) {
        let cause = profile_cause_summary(&raw);
        return Err(format!(
            "{context}: blocked: refusing database writes for smoke/tests — isolated test-world profile required (ADR-0004).\n\
             \n\
             Cause: `{PROFILE_ENV_VAR}` is {cause}.\n\
             \n\
             {STANDARD_REMEDIATION_TARGETS}\
             \n\
             Fix: `export {PROFILE_ENV_VAR}={ISOLATED_PROFILE_VALUE}` and point `DOLT_DB` / `DOLT_SOCKET` (or TCP `DOLT_HOST`/`DOLT_PORT`) at a **disposable** Dolt database, not the curated canonical checkout catalog.",
        ));
    }

    check_test_world_expectation(&config.database)?;
    resolved_database_identity_allows_writes(config).map_err(|identity_cause| {
        format!(
            "{context}: blocked: refusing database writes — resolved connection identity fails isolated test-world policy (ADR-0004).\n\
             \n\
             Cause: {identity_cause}\n\
             Resolved target: database={:?}, socket={}, host={}:{}\n\
             \n\
             {STANDARD_REMEDIATION_TARGETS}\
             \n\
             Fix: export a disposable `{PROJECT_SLUG_ENV_VAR}`-distinct database name (`DOLT_DB=coherence_test_<uuid>` is typical under user-scoped Dolt) or widen `{TEST_WORLD_ALLOWLIST_ENV_VAR}` deliberately; unset `{PROFILE_ENV_VAR}` is never sufficient alone.",
            config.database,
            config.socket_path.display(),
            config.host,
            config.port
        )
    })
}

/// Same policy as [`require_isolated_test_world_for_writes`] but panic with the refusal text (crate unit tests).
#[cfg(test)]
#[track_caller]
pub fn panic_unless_isolated_test_world_for_writes(context: &str, config: &ConnectionConfig) {
    if let Err(msg) = require_isolated_test_world_for_writes(context, config) {
        panic!("{msg}");
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn dummy_config(database: impl Into<String>) -> ConnectionConfig {
        ConnectionConfig {
            socket_path: PathBuf::from("/tmp/coherence-test.sock"),
            host: "127.0.0.1".into(),
            port: 3306,
            user: "root".into(),
            password: None,
            database: database.into(),
        }
    }

    fn clear_guard_env() {
        env::remove_var(PROFILE_ENV_VAR);
        env::remove_var(PROJECT_SLUG_ENV_VAR);
        env::remove_var(TEST_WORLD_ALLOWLIST_ENV_VAR);
        env::remove_var(TEST_WORLD_EXPECTED_DB_ENV_VAR);
        env::remove_var("COHERENCE_TEST_DB_PREFIX");
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
    }

    #[test]
    fn refusal_includes_stable_next_targets_when_unset() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_guard_env();
        let config = dummy_config("irrelevant");
        let msg = require_isolated_test_world_for_writes("test_ctx", &config).unwrap_err();
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
    fn ok_when_profile_test_and_disposable_db() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_guard_env();
        env::set_var(PROFILE_ENV_VAR, "test");
        env::set_var("COHERENCE_USE_USER_SCOPED_DOLT", "1");
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let config = dummy_config(format!("coherence_test_{uuid}"));
        require_isolated_test_world_for_writes("test_ctx", &config).unwrap();
    }

    #[test]
    fn ok_repo_local_catalog_name_when_user_scoped_dolt_disabled() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_guard_env();
        env::set_var(PROFILE_ENV_VAR, "test");
        let config = dummy_config("coherence-core-db");
        require_isolated_test_world_for_writes("test_ctx", &config).unwrap();
    }

    #[test]
    fn refusal_when_profile_non_test() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_guard_env();
        env::set_var(PROFILE_ENV_VAR, "canonical");
        let config = dummy_config("coherence_test_x");
        let msg = require_isolated_test_world_for_writes("test_ctx", &config).unwrap_err();
        assert!(msg.contains("non-test profile"), "unexpected: {msg}");
    }

    #[test]
    fn refuse_when_slug_set_and_database_equals_canonical_even_if_profile_test() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_guard_env();
        env::set_var("COHERENCE_USE_USER_SCOPED_DOLT", "1");
        env::set_var(PROFILE_ENV_VAR, "test");
        env::set_var(PROJECT_SLUG_ENV_VAR, "Coherence-Core-Db");
        let config = dummy_config("coherence-core-db");
        let msg = require_isolated_test_world_for_writes("test_ctx", &config).unwrap_err();
        assert!(
            msg.contains(PROJECT_SLUG_ENV_VAR),
            "expected slug env hint: {msg}"
        );
        assert!(
            msg.contains("matches canonical"),
            "expected canonical match cause: {msg}"
        );
    }

    #[test]
    fn allow_when_slug_set_but_database_differs_even_without_prefix() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_guard_env();
        env::set_var("COHERENCE_USE_USER_SCOPED_DOLT", "1");
        env::set_var(PROFILE_ENV_VAR, "test");
        env::set_var(PROJECT_SLUG_ENV_VAR, "coherence-core-db");
        let config = dummy_config("my_private_throwaway_db");
        require_isolated_test_world_for_writes("test_ctx", &config).unwrap();
    }

    #[test]
    fn refuse_when_slug_unset_and_db_not_disposable() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_guard_env();
        env::set_var("COHERENCE_USE_USER_SCOPED_DOLT", "1");
        env::set_var(PROFILE_ENV_VAR, "test");
        let config = dummy_config("some_random_db_name");
        let msg = require_isolated_test_world_for_writes("test_ctx", &config).unwrap_err();
        assert!(
            msg.contains("not disposable"),
            "expected disposable policy: {msg}"
        );
    }

    #[test]
    fn allow_via_allowlist_when_slug_unset() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_guard_env();
        env::set_var("COHERENCE_USE_USER_SCOPED_DOLT", "1");
        env::set_var(PROFILE_ENV_VAR, "test");
        env::set_var(TEST_WORLD_ALLOWLIST_ENV_VAR, "staging_clone,fixture_db");
        let config = dummy_config("Fixture_Db");
        require_isolated_test_world_for_writes("test_ctx", &config).unwrap();
    }

    #[test]
    fn coherence_test_world_mismatch_errors() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_guard_env();
        env::set_var(PROFILE_ENV_VAR, "test");
        env::set_var(TEST_WORLD_EXPECTED_DB_ENV_VAR, "coherence_test_expected");
        let config = dummy_config("coherence_test_other");
        let msg = require_isolated_test_world_for_writes("test_ctx", &config).unwrap_err();
        assert!(
            msg.contains(TEST_WORLD_EXPECTED_DB_ENV_VAR),
            "unexpected: {msg}"
        );
    }
}
