//! Isolated test-world guard (ADR-0004): refuse smoke and integration-test writes unless
//! `COHERENCE_DB_PROFILE=test` and, on the **user-scoped shared Dolt** layout (ADR-0006), the resolved
//! [`ConnectionConfig`] points at a disposable or otherwise non-canonical database name (see [`require_isolated_test_world_for_writes`]).
//! Repo-local `.dolt` catalogs use the legacy **profile=test-only** gate so checkout workflows remain usable.
//!
//! When `.coherence/project.toml` binds **`project_hash`**, mutating tests/smoke must not target the
//! manifest-derived **dev** tier catalog (`normalize(slug, hash, dev)`) — see [`refuse_manifest_bound_dev_catalog_for_test_writes`].

use std::env;

use crate::db::ConnectionConfig;
use crate::project_manifest::{self, CoherenceEnv};

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
    let manifest = project_manifest::try_read_project_manifest_from_cwd();
    crate::db::user_scoped_dolt_from_manifest(&manifest)
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
        return Ok(());
    }

    if database_has_disposable_prefix(database) {
        return Ok(());
    }

    if database_is_explicitly_allowlisted(database) {
        return Ok(());
    }

    let prefix = configured_test_db_prefix();
    Err(format!(
        "resolved `DOLT_DB`={database:?} is not disposable (expected prefix {prefix:?} via `COHERENCE_TEST_DB_PREFIX` or an entry in `{TEST_WORLD_ALLOWLIST_ENV_VAR}`), and `{PROJECT_SLUG_ENV_VAR}` is unset — cannot prove a non-canonical disposable target",
    ))
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

/// When the cwd manifest binds [`project_manifest::ProjectManifest::project_hash`], refuse mutating
/// workflows that resolve to the **dev** tier catalog name for that slug/hash, so `COHERENCE_ENV=test`
/// (and an unset `DOLT_DB`) cannot be implied while still pointing at `…_dev` (COREDB-zx5).
fn refuse_manifest_bound_dev_catalog_for_test_writes(
    context: &str,
    config: &ConnectionConfig,
) -> Result<(), String> {
    let Some(manifest) = project_manifest::try_read_project_manifest_from_cwd() else {
        return Ok(());
    };
    let Some(ref hash_raw) = manifest.project_hash else {
        return Ok(());
    };
    let hash = hash_raw.trim();
    if hash.is_empty() {
        return Ok(());
    }
    let slug = manifest.project_slug.trim();
    if slug.is_empty() {
        return Ok(());
    }
    let dev_catalog =
        project_manifest::effective_dolt_catalog_name(slug, Some(hash), CoherenceEnv::Dev)?;
    let database = config.database.as_str();
    if !database.eq_ignore_ascii_case(dev_catalog.as_str()) {
        return Ok(());
    }
    if database_is_explicitly_allowlisted(database) {
        return Ok(());
    }
    let env_name = project_manifest::COHERENCE_ENV_VAR;
    Err(format!(
        "{context}: blocked: refusing database writes — isolated test profile must not target the manifest-bound **dev** catalog (ADR-0004).\n\
         \n\
         Cause: resolved `DOLT_DB`={database:?} matches dev-tier name {dev_catalog:?} derived from `{env_name}=dev` rules; use the **test** tier or a disposable database.\n\
         Resolved target: database={database:?}, socket={sock}, host={host}:{port}\n\
         \n\
         {remediation}\
         \n\
         Fix: export `{env_name}=test` (as in `scripts/with-isolated-test-profile`) and avoid overriding `DOLT_DB` with the dev catalog, or use a disposable `coherence_test_*` name / `{allow_var}`.",
        context = context,
        database = database,
        dev_catalog = dev_catalog,
        env_name = env_name,
        sock = config.socket_path.display(),
        host = config.host,
        port = config.port,
        remediation = STANDARD_REMEDIATION_TARGETS,
        allow_var = TEST_WORLD_ALLOWLIST_ENV_VAR,
    ))
}

/// Returns `Ok(())` when this process may run mutating smoke or test workflows against the resolved [`ConnectionConfig`].
///
/// `context` labels the caller in errors (for example `m0-smoke` or a test module path).
///
/// # Errors
///
/// Returns an error message when the config resolves to the canonical dev/prod catalog or the
/// `COHERENCE_DB_PROFILE` environment variable is not set to `test`.
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
    refuse_manifest_bound_dev_catalog_for_test_writes(context, config)?;
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
///
/// # Panics
///
/// Panics if the config resolves to the canonical dev/prod catalog and context is not test.
#[track_caller]
pub fn panic_unless_isolated_test_world_for_writes(context: &str, config: &ConnectionConfig) {
    if let Err(msg) = require_isolated_test_world_for_writes(context, config) {
        panic!("{msg}");
    }
}

/// Serialize `std::env` access for unit tests: [`lock_test_env`] must bracket guard scenarios and
/// any test that reads [`ConnectionConfig::from_env`] against a real Dolt catalog.
static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Hold for the duration of any test that calls [`ConnectionConfig::from_env`] against a real Dolt
/// target so guard tests cannot transient `remove_var`/`set_var` between load and connect.
pub fn lock_test_env() -> std::sync::MutexGuard<'static, ()> {
    TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub struct EnvConnLock<T> {
    #[allow(clippy::pub_underscore_fields)]
    pub _lock: std::sync::MutexGuard<'static, ()>,
    pub inner: T,
}

impl<T> std::ops::Deref for EnvConnLock<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> std::ops::DerefMut for EnvConnLock<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use crate::project_manifest;
    use std::path::PathBuf;

    /// Keys touched by [`clear_guard_env`] or guard scenarios that must be restored so parallel
    /// workspace tests see the `with-isolated-test-profile` session again after each guard case.
    const SNAPSHOT_KEYS: &[&str] = &[
        PROFILE_ENV_VAR,
        PROJECT_SLUG_ENV_VAR,
        TEST_WORLD_ALLOWLIST_ENV_VAR,
        TEST_WORLD_EXPECTED_DB_ENV_VAR,
        "COHERENCE_TEST_DB_PREFIX",
        "COHERENCE_ENV",
        "DOLT_DB",
        "DOLT_SOCKET",
        "DOLT_HOST",
        "DOLT_PORT",
        "COHERENCE_DOLT_TCP_PORT",
        "DOLT_USER",
        "DOLT_PASSWORD",
        "COHERENCE_DOLT_RUNTIME_DIR",
    ];

    struct SaveCwd(std::path::PathBuf);

    impl SaveCwd {
        fn chdir(path: &std::path::Path) -> Self {
            let prev = env::current_dir().expect("cwd");
            env::set_current_dir(path).expect("chdir");
            Self(prev)
        }
    }

    impl Drop for SaveCwd {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }

    fn tmp_git_repo() -> tempfile::TempDir {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        tmp
    }

    struct SavedTestEnv {
        pairs: Vec<(String, Option<String>)>,
    }

    impl SavedTestEnv {
        fn snapshot(keys: &[&str]) -> Self {
            let pairs = keys
                .iter()
                .map(|k| ((*k).to_string(), env::var(k).ok()))
                .collect();
            Self { pairs }
        }
    }

    impl Drop for SavedTestEnv {
        fn drop(&mut self) {
            for (k, v) in &self.pairs {
                match v {
                    Some(val) => env::set_var(k, val),
                    None => env::remove_var(k),
                }
            }
        }
    }

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
        env::remove_var("COHERENCE_ENV");
        env::remove_var("DOLT_DB");
    }

    /// Test harness that locks env, snapshots env, clears guard vars, creates a temporary git repo
    /// with the given manifest, chdirs into it, and sets `COHERENCE_DB_PROFILE=test`.
    struct TestHarness {
        _lock: std::sync::MutexGuard<'static, ()>,
        _restore: SavedTestEnv,
        _tmp: tempfile::TempDir,
        _cwd: SaveCwd,
    }

    impl TestHarness {
        fn with_manifest(body: &str) -> Self {
            let _lock = lock_test_env();
            let _restore = SavedTestEnv::snapshot(SNAPSHOT_KEYS);
            clear_guard_env();
            let _tmp = tmp_git_repo();
            std::fs::create_dir_all(_tmp.path().join(".coherence")).unwrap();
            std::fs::write(_tmp.path().join(".coherence/project.toml"), body).unwrap();
            let _cwd = SaveCwd::chdir(_tmp.path());
            env::set_var(PROFILE_ENV_VAR, "test");
            Self {
                _lock,
                _restore,
                _tmp,
                _cwd,
            }
        }
    }

    #[test]
    fn refusal_includes_stable_next_targets_when_unset() {
        let _lock = lock_test_env();
        let _restore = SavedTestEnv::snapshot(SNAPSHOT_KEYS);
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
        let _harness = TestHarness::with_manifest(
            r#"version = 2
project_slug = "fixture_project"
dolt_mode = "user-scoped"
"#,
        );
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let config = dummy_config(format!("coherence_test_{uuid}"));
        require_isolated_test_world_for_writes("test_ctx", &config).unwrap();
    }

    #[test]
    fn ok_repo_local_catalog_name_when_user_scoped_dolt_disabled() {
        let _harness = TestHarness::with_manifest(
            r#"version = 2
project_slug = "coherence-core-db"
dolt_mode = "repo-local"
"#,
        );
        let config = dummy_config("coherence-core-db");
        require_isolated_test_world_for_writes("test_ctx", &config).unwrap();
    }

    #[test]
    fn refusal_when_profile_non_test() {
        let _lock = lock_test_env();
        let _restore = SavedTestEnv::snapshot(SNAPSHOT_KEYS);
        clear_guard_env();
        env::set_var(PROFILE_ENV_VAR, "canonical");
        let config = dummy_config("coherence_test_x");
        let msg = require_isolated_test_world_for_writes("test_ctx", &config).unwrap_err();
        assert!(msg.contains("non-test profile"), "unexpected: {msg}");
    }

    #[test]
    fn refuse_when_slug_set_and_database_equals_canonical_even_if_profile_test() {
        let _harness = TestHarness::with_manifest(
            r#"version = 2
project_slug = "coherence-core-db"
dolt_mode = "user-scoped"
"#,
        );
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
        let _harness = TestHarness::with_manifest(
            r#"version = 2
project_slug = "coherence-core-db"
dolt_mode = "user-scoped"
"#,
        );
        env::set_var(PROJECT_SLUG_ENV_VAR, "coherence-core-db");
        let config = dummy_config("my_private_throwaway_db");
        require_isolated_test_world_for_writes("test_ctx", &config).unwrap();
    }

    #[test]
    fn refuse_when_slug_unset_and_db_not_disposable() {
        let _harness = TestHarness::with_manifest(
            r#"version = 2
project_slug = "fixture_project"
dolt_mode = "user-scoped"
"#,
        );
        let config = dummy_config("some_random_db_name");
        let msg = require_isolated_test_world_for_writes("test_ctx", &config).unwrap_err();
        assert!(
            msg.contains("not disposable"),
            "expected disposable policy: {msg}"
        );
    }

    #[test]
    fn allow_via_allowlist_when_slug_unset() {
        let _harness = TestHarness::with_manifest(
            r#"version = 2
project_slug = "fixture_project"
dolt_mode = "user-scoped"
"#,
        );
        env::set_var(TEST_WORLD_ALLOWLIST_ENV_VAR, "staging_clone,fixture_db");
        let config = dummy_config("Fixture_Db");
        require_isolated_test_world_for_writes("test_ctx", &config).unwrap();
    }

    #[test]
    fn coherence_test_world_mismatch_errors() {
        let _lock = lock_test_env();
        let _restore = SavedTestEnv::snapshot(SNAPSHOT_KEYS);
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

    #[test]
    fn refuse_when_resolved_db_matches_manifest_dev_catalog_even_if_coherence_env_test() {
        let _harness = TestHarness::with_manifest(
            r#"version = 2
project_slug = "svc"
project_hash = "cafe"
dolt_mode = "user-scoped"
"#,
        );
        env::set_var("COHERENCE_ENV", "test");
        let config = dummy_config("svc_cafe_dev");
        let msg = require_isolated_test_world_for_writes("test_ctx", &config).unwrap_err();
        assert!(
            msg.contains("dev") && msg.contains("ADR-0004"),
            "unexpected: {msg}"
        );
        assert!(
            msg.contains(project_manifest::COHERENCE_ENV_VAR),
            "expected COHERENCE_ENV in remediation: {msg}"
        );
    }

    #[test]
    fn ok_when_resolved_db_matches_manifest_test_catalog() {
        let _harness = TestHarness::with_manifest(
            r#"version = 2
project_slug = "svc"
project_hash = "cafe"
dolt_mode = "repo-local"
"#,
        );
        env::set_var("COHERENCE_ENV", "test");
        let config = dummy_config("svc_cafe_test");
        require_isolated_test_world_for_writes("test_ctx", &config).unwrap();
    }

    #[test]
    fn manifest_dev_catalog_allowed_when_explicitly_allowlisted() {
        let _harness = TestHarness::with_manifest(
            r#"version = 2
project_slug = "svc"
project_hash = "cafe"
dolt_mode = "user-scoped"
"#,
        );
        env::set_var(TEST_WORLD_ALLOWLIST_ENV_VAR, "svc_cafe_dev");
        let config = dummy_config("svc_cafe_dev");
        require_isolated_test_world_for_writes("test_ctx", &config).unwrap();
    }
}
