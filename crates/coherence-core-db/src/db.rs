use std::env;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use mysql::prelude::Queryable;
use mysql::{Conn, OptsBuilder};

use crate::models::{AcceptanceCriterion, Spec};
use crate::project_manifest;
use crate::spec_store;

/// Environment variable holding the logical MySQL/Dolt database name.
pub(crate) const DOLT_DB_ENV: &str = "DOLT_DB";
/// Populated from `.coherence/project.toml` when unset so ADR-0004 isolation checks align with manifest identity.
const COHERENCE_PROJECT_SLUG_ENV: &str = "COHERENCE_PROJECT_SLUG";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    Socket,
    TcpFallback,
}

impl Display for ConnectionMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Socket => write!(f, "socket"),
            Self::TcpFallback => write!(f, "tcp_fallback"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub socket_path: PathBuf,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Option<String>,
    pub database: String,
}

/// ADR-0006 explicit opt-in: coordinated defaults for one user-scoped `dolt sql-server`.
#[must_use]
pub fn user_scoped_dolt_from_env() -> bool {
    matches!(
        env::var("COHERENCE_USE_USER_SCOPED_DOLT").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

fn runtime_coherence_dir() -> PathBuf {
    if let Ok(extra) = env::var("COHERENCE_DOLT_RUNTIME_DIR") {
        PathBuf::from(extra)
    } else if let Ok(runtime) = env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime).join("coherence")
    } else {
        let home = env::var("HOME").unwrap_or_else(|_| "/".to_string());
        PathBuf::from(home).join(".cache/coherence/run")
    }
}

#[must_use]
pub fn user_scoped_socket_default_path() -> PathBuf {
    runtime_coherence_dir().join("dolt.sock")
}

/// True when non-empty **`DOLT_DB`** is set (logical catalog explicitly selected).
#[must_use]
pub fn explicit_dolt_database_from_env() -> bool {
    env::var(DOLT_DB_ENV)
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

const USER_SCOPED_INTERNAL_TCP_PORT: u16 = 33_306;

impl ConnectionConfig {
    /// Build connection settings from environment and (when `DOLT_DB` is unset) the git-root manifest.
    ///
    /// Returns [`Err`] when [`project_manifest::coherence_env_from_std_env`] rejects **`COHERENCE_ENV`**
    /// or when a parsable `.coherence/project.toml` lacks both a bound **`project_hash`** and legacy
    /// **`dolt_db_name`** (run `project init` or set `DOLT_DB`).
    pub fn from_env() -> Result<Self, String> {
        let user_scoped = user_scoped_dolt_from_env();
        let coherence_env = project_manifest::coherence_env_from_std_env()?;

        let manifest = project_manifest::try_read_project_manifest_from_cwd();
        if let Some(ref m) = manifest {
            if env::var_os(COHERENCE_PROJECT_SLUG_ENV).is_none()
                && !m.project_slug.trim().is_empty()
            {
                env::set_var(COHERENCE_PROJECT_SLUG_ENV, m.project_slug.trim());
            }
        }

        let socket_path = env::var("DOLT_SOCKET")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                if user_scoped {
                    user_scoped_socket_default_path()
                } else {
                    PathBuf::from(".dolt/dolt.sock")
                }
            });

        let host = env::var("DOLT_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

        let port = env::var("DOLT_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or_else(|| {
                if user_scoped {
                    env::var("COHERENCE_DOLT_TCP_PORT")
                        .ok()
                        .and_then(|value| value.parse::<u16>().ok())
                        .unwrap_or(USER_SCOPED_INTERNAL_TCP_PORT)
                } else {
                    3306
                }
            });

        let user = env::var("DOLT_USER").unwrap_or_else(|_| "root".to_string());
        let password = env::var("DOLT_PASSWORD").ok();
        let database = resolve_effective_database_name(manifest.as_ref(), coherence_env)?;

        Ok(Self {
            socket_path,
            host,
            port,
            user,
            password,
            database,
        })
    }
}

/// Resolve the logical MySQL/Dolt database name from the manifest (**ignores **`DOLT_DB`**).
///
/// When the manifest includes a non-empty **`project_hash`**, the name is
/// [`project_manifest::effective_dolt_catalog_name`]. Legacy **`dolt_db_name`** is used only when no
/// non-empty **`project_hash`** is set on the manifest.
pub fn manifest_bound_catalog_name(
    manifest: &project_manifest::ProjectManifest,
    coherence_env: project_manifest::CoherenceEnv,
) -> Result<String, String> {
    let slug = manifest.project_slug.trim();
    if let Some(ref h) = manifest.project_hash {
        let ht = h.trim();
        if !ht.is_empty() {
            return project_manifest::effective_dolt_catalog_name(slug, Some(ht), coherence_env);
        }
    }

    if let Some(ref db) = manifest.dolt_db_name {
        let t = db.trim();
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }

    Err(
        "connection: `.coherence/project.toml` has no catalog binding (`project_hash` from `project init`, or legacy `dolt_db_name`). \
         Fix: run `project init`, or set `DOLT_DB`."
            .to_string(),
    )
}

/// Effective catalog **`ConnectionConfig`** would use when **`DOLT_DB`** were unset (**mirrors**
/// [`ConnectionConfig::from_env`] without reading **`DOLT_DB`**).
///
/// Parses **`COHERENCE_ENV`** against the OS environment — invalid values propagate as **`Err`**.
pub fn hypothetical_effective_catalog_from_cwd() -> Result<String, String> {
    let coherence_env = project_manifest::coherence_env_from_std_env()?;
    let manifest = project_manifest::try_read_project_manifest_from_cwd();
    resolve_effective_catalog_without_explicit_dolt_override(manifest.as_ref(), coherence_env)
}

/// **`DOLT_DB` env omitted** (`ConnectionConfig`-style basename / manifest semantics only).
fn resolve_effective_catalog_without_explicit_dolt_override(
    manifest: Option<&project_manifest::ProjectManifest>,
    coherence_env: project_manifest::CoherenceEnv,
) -> Result<String, String> {
    let Some(m) = manifest else {
        return Ok(default_database_name());
    };
    manifest_bound_catalog_name(m, coherence_env)
}

/// Manifest + git prerequisites used by **`db-ping`**, **`migrate`**, and **`scripts/dolt-start`**
/// before touching Dolt (**when **`DOLT_DB`** override is inactive**): git root, readable manifest,
/// **`project_slug`**, catalog binding (**`project_hash`** or **`dolt_db_name`**).
///
/// Returns **`Ok`** when **`explicit_dolt_database_from_env`** is true (**skip manifest path**).
pub fn manifest_catalog_rules_without_dolt_db() -> Result<(), String> {
    if explicit_dolt_database_from_env() {
        return Ok(());
    }

    project_manifest::coherence_env_from_std_env()?;

    let cwd = env::current_dir().map_err(|e| format!("cannot read working directory: {e}"))?;
    let Some(repo_root) = project_manifest::find_git_repo_root(cwd) else {
        return Err(
            "not inside a git work tree (.git not found upstream of cwd).\n\
             Fix: cd into your Coherence project repository, or set DOLT_DB to select a logical catalog explicitly."
                .to_string(),
        );
    };

    let manifest_path = project_manifest::coherence_manifest_path(&repo_root);
    if !manifest_path.is_file() {
        return Err(format!(
            "missing project manifest `{}`.\n\
             Fix: coherence-core-db project init --slug YOUR_SLUG (binds `project_hash` once the repo identity is pinned; see AGENTS.md § Project identity and manifest lifecycle)",
            manifest_path.display()
        ));
    }

    let manifest = project_manifest::read_manifest(&repo_root).map_err(|e| {
        format!(
            "invalid project manifest ({}): {e}",
            manifest_path.display()
        )
    })?;

    manifest_bound_catalog_name(&manifest, project_manifest::coherence_env_from_std_env()?)?;
    Ok(())
}

pub fn manifest_catalog_preflight_for_connect(context: &str) -> Result<(), String> {
    if explicit_dolt_database_from_env() {
        return Ok(());
    }
    manifest_catalog_rules_without_dolt_db().map_err(|detail| format!("{context}: {detail}"))
}

/// Resolve the logical MySQL/Dolt database name: non-empty **`DOLT_DB`** wins; else manifest at the
/// git root of [`std::env::current_dir`].
///
/// When the manifest includes a non-empty **`project_hash`**, the name is
/// [`project_manifest::effective_dolt_catalog_name`] (**`project_slug`**, hash, **`COHERENCE_ENV`** tier)
/// — legacy **`dolt_db_name`** alone is not used on that path. Without a hash, a non-empty
/// **`dolt_db_name`** is still accepted. With no manifest (or unreadable/missing file), falls back to
/// [`default_database_name`].
fn resolve_effective_database_name(
    manifest: Option<&project_manifest::ProjectManifest>,
    coherence_env: project_manifest::CoherenceEnv,
) -> Result<String, String> {
    if let Ok(db) = env::var(DOLT_DB_ENV) {
        let t = db.trim();
        if !t.is_empty() {
            return Ok(db);
        }
    }
    resolve_effective_catalog_without_explicit_dolt_override(manifest, coherence_env)
}

fn default_database_name() -> String {
    env::current_dir()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "dolt".to_string())
}

fn socket_opts(config: &ConnectionConfig) -> OptsBuilder {
    OptsBuilder::new()
        .user(Some(config.user.clone()))
        .pass(config.password.clone())
        .db_name(Some(config.database.clone()))
        .socket(Some(config.socket_path.to_string_lossy().to_string()))
}

fn tcp_opts(config: &ConnectionConfig) -> OptsBuilder {
    OptsBuilder::new()
        .user(Some(config.user.clone()))
        .pass(config.password.clone())
        .db_name(Some(config.database.clone()))
        .ip_or_hostname(Some(config.host.clone()))
        .tcp_port(config.port)
}

fn socket_opts_no_db(config: &ConnectionConfig) -> OptsBuilder {
    OptsBuilder::new()
        .user(Some(config.user.clone()))
        .pass(config.password.clone())
        .db_name(None::<String>)
        .socket(Some(config.socket_path.to_string_lossy().to_string()))
}

fn tcp_opts_no_db(config: &ConnectionConfig) -> OptsBuilder {
    OptsBuilder::new()
        .user(Some(config.user.clone()))
        .pass(config.password.clone())
        .db_name(None::<String>)
        .ip_or_hostname(Some(config.host.clone()))
        .tcp_port(config.port)
}

pub fn connect_without_database(
    config: &ConnectionConfig,
) -> Result<(Conn, ConnectionMode), String> {
    match Conn::new(socket_opts_no_db(config)) {
        Ok(conn) => Ok((conn, ConnectionMode::Socket)),
        Err(socket_err) => {
            eprintln!(
                "connection: socket failed at {} ({socket_err})",
                config.socket_path.display()
            );
            match Conn::new(tcp_opts_no_db(config)) {
                Ok(conn) => Ok((conn, ConnectionMode::TcpFallback)),
                Err(tcp_err) => Err(format!(
                    "connection failed: socket={} then tcp={}:{} ({tcp_err})",
                    config.socket_path.display(),
                    config.host,
                    config.port,
                )),
            }
        }
    }
}

/// Runs `SELECT 1` without selecting `ConnectionConfig.database` first (server readiness).
pub fn ping_server(config: &ConnectionConfig) -> Result<ConnectionMode, String> {
    let (mut conn, mode) = connect_without_database(config)?;
    let _: Option<u8> = conn
        .query_first("SELECT 1")
        .map_err(|err| format!("ping query failed: {err}"))?;
    Ok(mode)
}

#[must_use]
pub fn mysql_quote_identifier(name: &str) -> String {
    let escaped = name.replace('`', "``");
    format!("`{escaped}`")
}

/// Ensures `config.database` exists on the server (`CREATE DATABASE IF NOT EXISTS`).
///
/// Runs for **repo-local** `.dolt` sockets as well as user-scoped Dolt (ADR-0006): after
/// `project init`, the manifest catalog name must exist before `migrate` / Refinery connect.
pub fn ensure_project_database(config: &ConnectionConfig) -> Result<(), String> {
    if config.database.is_empty() {
        return Err("logical catalog name is empty; cannot ensure database".to_string());
    }
    let (mut conn, _) = connect_without_database(config)?;
    let ident = mysql_quote_identifier(&config.database);
    let stmt = format!("CREATE DATABASE IF NOT EXISTS {ident}");
    conn.query_drop(stmt)
        .map_err(|err| format!("failed to create database {}: {err}", config.database))?;
    Ok(())
}

pub fn connect(config: &ConnectionConfig) -> Result<(Conn, ConnectionMode), String> {
    match Conn::new(socket_opts(config)) {
        Ok(conn) => Ok((conn, ConnectionMode::Socket)),
        Err(socket_err) => {
            eprintln!(
                "connection: socket failed at {} ({socket_err})",
                config.socket_path.display()
            );
            match Conn::new(tcp_opts(config)) {
                Ok(conn) => Ok((conn, ConnectionMode::TcpFallback)),
                Err(tcp_err) => Err(format!(
                    "connection failed: socket={} then tcp={}:{} ({tcp_err})",
                    config.socket_path.display(),
                    config.host,
                    config.port,
                )),
            }
        }
    }
}

pub fn insert_spec(conn: &mut Conn, spec: &Spec) -> Result<(), String> {
    let mut normalized = spec.clone();
    if normalized.slug.is_empty() {
        normalized.slug = normalized.id.to_ascii_lowercase();
    }
    if normalized.description.is_empty() {
        normalized.description = "m0-smoke spec".to_string();
    }
    if normalized.created_at.is_empty() {
        normalized.created_at = "m0".to_string();
    }
    if normalized.updated_at.is_empty() {
        normalized.updated_at = "m0".to_string();
    }
    spec_store::put_spec(conn, &normalized)
}

pub fn insert_acceptance_criterion(
    conn: &mut Conn,
    ac: &AcceptanceCriterion,
) -> Result<(), String> {
    let mut normalized = ac.clone();
    if normalized.slug.is_empty() {
        normalized.slug = crate::models::slug_from_id(&normalized.id);
    }
    if normalized.intent.is_empty() {
        normalized.intent = "m0-smoke intent".to_string();
    }
    if normalized.created_at.is_empty() {
        normalized.created_at = "m0".to_string();
    }
    if normalized.updated_at.is_empty() {
        normalized.updated_at = "m0".to_string();
    }
    spec_store::put_acceptance_criterion(conn, &normalized)
}

pub fn counts(conn: &mut Conn) -> Result<(u64, u64), String> {
    let spec_count: Option<u64> = conn
        .query_first("SELECT COUNT(*) FROM specs")
        .map_err(|err| format!("failed to count specs: {err}"))?;
    let ac_count: Option<u64> = conn
        .query_first("SELECT COUNT(*) FROM acceptance_criteria")
        .map_err(|err| format!("failed to count acceptance_criteria: {err}"))?;
    Ok((spec_count.unwrap_or(0), ac_count.unwrap_or(0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mysql_quote_identifier_escapes_backticks() {
        assert_eq!(mysql_quote_identifier("a"), "`a`");
        assert_eq!(mysql_quote_identifier("a`b"), "`a``b`");
    }
}

#[cfg(test)]
mod connection_config_manifest_tests {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;

    struct MultiEnvRestore {
        pairs: Vec<(&'static str, Option<String>)>,
    }

    impl MultiEnvRestore {
        fn snapshot(keys: &[&'static str]) -> Self {
            Self {
                pairs: keys.iter().map(|&k| (k, env::var(k).ok())).collect(),
            }
        }
    }

    impl Drop for MultiEnvRestore {
        fn drop(&mut self) {
            for (k, v) in &self.pairs {
                match v {
                    Some(val) => env::set_var(k, val),
                    None => env::remove_var(k),
                }
            }
        }
    }

    struct SaveCwd(PathBuf);

    impl SaveCwd {
        fn chdir(path: &Path) -> Self {
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

    fn write_manifest(repo: &Path, dolt_db_line: &str) {
        fs::create_dir_all(repo.join(".coherence")).unwrap();
        fs::write(
            repo.join(".coherence/project.toml"),
            format!("version = 1\nproject_slug = \"myproj\"\n{dolt_db_line}",),
        )
        .unwrap();
    }

    fn tmp_git_repo() -> TempDir {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        tmp
    }

    const ENV_FOR_CONNECTION_TESTS: &[&str] = &[
        "DOLT_DB",
        "COHERENCE_PROJECT_SLUG",
        "COHERENCE_USE_USER_SCOPED_DOLT",
        "COHERENCE_ENV",
    ];

    #[test]
    fn explicit_dolt_db_env_overrides_manifest() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
        env::remove_var("COHERENCE_ENV");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "dolt_db_name = \"from_manifest\"\n");
        let nested = tmp.path().join("deep/nested");
        fs::create_dir_all(&nested).unwrap();
        let _cwd = SaveCwd::chdir(&nested);
        env::set_var("DOLT_DB", "from_env");
        let cfg = ConnectionConfig::from_env().expect("from_env");
        assert_eq!(cfg.database, "from_env");
    }

    #[test]
    fn nested_cwd_resolves_manifest_database_name() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
        env::remove_var("COHERENCE_ENV");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "dolt_db_name = \"frozen_catalog_abc1\"\n");
        let nested = tmp.path().join("deep/nested");
        fs::create_dir_all(&nested).unwrap();
        let _cwd = SaveCwd::chdir(&nested);
        let cfg = ConnectionConfig::from_env().expect("from_env");
        assert_eq!(cfg.database, "frozen_catalog_abc1");
    }

    #[test]
    fn from_env_sets_coherence_project_slug_from_manifest_when_unset() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
        env::remove_var("COHERENCE_ENV");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "dolt_db_name = \"x\"\n");
        let _cwd = SaveCwd::chdir(tmp.path());
        let _cfg = ConnectionConfig::from_env().expect("from_env");
        assert_eq!(env::var("COHERENCE_PROJECT_SLUG").unwrap(), "myproj");
    }

    #[test]
    fn project_hash_dominates_legacy_dolt_db_name() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
        env::remove_var("COHERENCE_ENV");

        let tmp = tmp_git_repo();
        fs::create_dir_all(tmp.path().join(".coherence")).unwrap();
        fs::write(
            tmp.path().join(".coherence/project.toml"),
            r#"version = 2
project_slug = "svc"
project_hash = "cafe"
dolt_db_name = "legacy_only_ignored"
"#,
        )
        .unwrap();
        let _cwd = SaveCwd::chdir(tmp.path());
        let cfg = ConnectionConfig::from_env().expect("from_env");
        assert_eq!(cfg.database, "svc_cafe_dev");
    }

    #[test]
    fn coherence_env_selects_distinct_effective_catalog() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");

        let tmp = tmp_git_repo();
        fs::create_dir_all(tmp.path().join(".coherence")).unwrap();
        fs::write(
            tmp.path().join(".coherence/project.toml"),
            r#"version = 2
project_slug = "svc"
project_hash = "cafe"
"#,
        )
        .unwrap();
        let _cwd = SaveCwd::chdir(tmp.path());

        env::set_var("COHERENCE_ENV", "test");
        let test_catalog = ConnectionConfig::from_env().expect("from_env").database;
        assert_eq!(test_catalog, "svc_cafe_test");

        env::set_var("COHERENCE_ENV", "dev");
        let dev_catalog = ConnectionConfig::from_env().expect("from_env").database;
        assert_eq!(dev_catalog, "svc_cafe_dev");
        assert_ne!(test_catalog, dev_catalog);
    }

    #[test]
    fn manifest_without_catalog_binding_errors() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
        env::remove_var("COHERENCE_ENV");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "");
        let _cwd = SaveCwd::chdir(tmp.path());
        let err = ConnectionConfig::from_env().unwrap_err();
        assert!(
            err.contains("project init") && err.contains("DOLT_DB"),
            "{err}"
        );
    }

    #[test]
    fn invalid_coherence_env_errors_before_manifest_resolution() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
        env::set_var("COHERENCE_ENV", "staging");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "dolt_db_name = \"x\"\n");
        let _cwd = SaveCwd::chdir(tmp.path());
        let err = ConnectionConfig::from_env().unwrap_err();
        assert!(err.contains("COHERENCE_ENV"), "{err}");
    }

    #[test]
    fn outside_git_worktree_uses_directory_basename_for_database() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
        env::remove_var("COHERENCE_ENV");

        let tmp = TempDir::new().unwrap();
        let _cwd = SaveCwd::chdir(tmp.path());
        let base = tmp
            .path()
            .file_name()
            .expect("file_name")
            .to_string_lossy()
            .into_owned();
        let cfg = ConnectionConfig::from_env().expect("from_env");
        assert_eq!(cfg.database, base);
    }

    #[test]
    fn manifest_catalog_preflight_errors_manifest_without_catalog_binding_in_git_repo() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
        env::remove_var("COHERENCE_ENV");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "");
        let _cwd = SaveCwd::chdir(tmp.path());
        let err = manifest_catalog_preflight_for_connect("migrate").unwrap_err();
        assert!(err.starts_with("migrate:"), "{err}");
        assert!(
            err.contains("project init") || err.contains("dolt_db_name"),
            "{err}"
        );
    }

    #[test]
    fn manifest_catalog_preflight_ok_when_only_project_hash_bound() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
        env::remove_var("COHERENCE_ENV");

        let tmp = tmp_git_repo();
        fs::create_dir_all(tmp.path().join(".coherence")).unwrap();
        fs::write(
            tmp.path().join(".coherence/project.toml"),
            r#"version = 2
project_slug = "svc"
project_hash = "cafe"
"#,
        )
        .unwrap();
        let _cwd = SaveCwd::chdir(tmp.path());
        manifest_catalog_preflight_for_connect("migrate").expect("preflight");
    }

    #[test]
    fn manifest_catalog_preflight_skips_manifest_when_explicit_dolt_db() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_ENV");
        env::set_var("COHERENCE_USE_USER_SCOPED_DOLT", "1");
        env::set_var("DOLT_DB", "explicit_db");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "");
        let _cwd = SaveCwd::chdir(tmp.path());
        manifest_catalog_preflight_for_connect("migrate").unwrap();
    }

    #[test]
    fn manifest_catalog_preflight_errors_git_repo_without_manifest_file() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
        env::remove_var("COHERENCE_ENV");

        let tmp = tmp_git_repo();
        let _cwd = SaveCwd::chdir(tmp.path());
        let err = manifest_catalog_preflight_for_connect("db-ping").unwrap_err();
        assert!(err.starts_with("db-ping:"), "{err}");
        assert!(
            err.contains("missing project manifest") || err.contains(".coherence"),
            "{err}"
        );
        assert!(
            err.contains("project init") || err.contains("--slug"),
            "{err}"
        );
    }

    #[test]
    fn manifest_catalog_preflight_errors_not_inside_git_when_dolt_db_unset() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
        env::remove_var("COHERENCE_ENV");

        let tmp = TempDir::new().unwrap();
        let _cwd = SaveCwd::chdir(tmp.path());
        let err = manifest_catalog_preflight_for_connect("db-ping").unwrap_err();
        assert!(err.contains("git work tree"), "{err}");
    }

    #[test]
    fn manifest_catalog_preflight_ok_legacy_dolt_db_name_only() {
        let _guard = crate::test_world_guard::lock_test_env();
        let _restore = MultiEnvRestore::snapshot(ENV_FOR_CONNECTION_TESTS);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");
        env::remove_var("COHERENCE_ENV");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "dolt_db_name = \"frozen_legacy\"\n");
        let _cwd = SaveCwd::chdir(tmp.path());
        manifest_catalog_preflight_for_connect("migrate").unwrap();
    }
}
