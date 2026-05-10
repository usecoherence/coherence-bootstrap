use std::env;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use mysql::prelude::Queryable;
use mysql::{Conn, OptsBuilder};

use crate::models::{AcceptanceCriterion, Spec};
use crate::project_manifest;
use crate::spec_store;

/// Environment variable holding the logical MySQL/Dolt database name.
const DOLT_DB_ENV: &str = "DOLT_DB";
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

const USER_SCOPED_INTERNAL_TCP_PORT: u16 = 33_306;

impl ConnectionConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let user_scoped = user_scoped_dolt_from_env();

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
        let database = resolve_database_name_with_manifest(manifest.as_ref());

        Self {
            socket_path,
            host,
            port,
            user,
            password,
            database,
        }
    }
}

/// Non-empty `DOLT_DB` wins; else `.coherence/project.toml` (`dolt_db_name`) at the git root of cwd; else [`default_database_name`].
fn resolve_database_name_with_manifest(
    manifest: Option<&project_manifest::ProjectManifest>,
) -> String {
    if let Ok(db) = env::var(DOLT_DB_ENV) {
        if !db.trim().is_empty() {
            return db;
        }
    }
    manifest
        .and_then(|m| m.dolt_db_name.as_ref())
        .filter(|s| !s.trim().is_empty())
        .map(std::string::ToString::to_string)
        .unwrap_or_else(default_database_name)
}

/// `migrate` only: refuses user-scoped runs without explicit `DOLT_DB` when a manifest exists but
/// `dolt_db_name` was never frozen (`project init`).
///
/// Call this **before** [`ConnectionConfig::from_env`] so the process never picks a stale default
/// catalog name for migration.
pub fn preflight_user_scoped_migrate_binding() -> Result<(), String> {
    if !user_scoped_dolt_from_env() {
        return Ok(());
    }
    if env::var(DOLT_DB_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        return Ok(());
    }
    let cwd =
        env::current_dir().map_err(|e| format!("migrate: cannot read working directory: {e}"))?;
    let Some(repo_root) = project_manifest::find_git_repo_root(cwd) else {
        return Ok(());
    };
    let path = project_manifest::coherence_manifest_path(&repo_root);
    if !path.is_file() {
        return Ok(());
    }
    let manifest = project_manifest::read_manifest(&repo_root).map_err(|e| {
        format!(
            "migrate: invalid project manifest ({}): {e}",
            path.display()
        )
    })?;
    let have_name = manifest
        .dolt_db_name
        .as_ref()
        .is_some_and(|s| !s.trim().is_empty());
    if !have_name {
        return Err(
            "migrate (user-scoped Dolt): `.coherence/project.toml` exists but `dolt_db_name` is not set.\n\
             Fix: run `cargo run -p coherence-core-db -- project init` (or set `DOLT_DB` explicitly)."
                .to_string(),
        );
    }
    Ok(())
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

/// Ensures `config.database` exists on the server (ADR-0006 multi-db `--data-dir` layout).
pub fn ensure_project_database(config: &ConnectionConfig) -> Result<(), String> {
    if !user_scoped_dolt_from_env() {
        return Ok(());
    }
    if config.database.is_empty() {
        return Err("DOLT_DB is empty; cannot ensure database".to_string());
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
    use std::sync::Mutex;

    use tempfile::TempDir;

    use super::*;

    static LOCK: Mutex<()> = Mutex::new(());

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

    #[test]
    fn explicit_dolt_db_env_overrides_manifest() {
        let _lock = LOCK.lock().unwrap();
        let _restore = MultiEnvRestore::snapshot(&[
            "DOLT_DB",
            "COHERENCE_PROJECT_SLUG",
            "COHERENCE_USE_USER_SCOPED_DOLT",
        ]);
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "dolt_db_name = \"from_manifest\"\n");
        let nested = tmp.path().join("deep/nested");
        fs::create_dir_all(&nested).unwrap();
        let _cwd = SaveCwd::chdir(&nested);
        env::set_var("DOLT_DB", "from_env");
        let cfg = ConnectionConfig::from_env();
        assert_eq!(cfg.database, "from_env");
    }

    #[test]
    fn nested_cwd_resolves_manifest_database_name() {
        let _lock = LOCK.lock().unwrap();
        let _restore = MultiEnvRestore::snapshot(&[
            "DOLT_DB",
            "COHERENCE_PROJECT_SLUG",
            "COHERENCE_USE_USER_SCOPED_DOLT",
        ]);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "dolt_db_name = \"frozen_catalog_abc1\"\n");
        let nested = tmp.path().join("deep/nested");
        fs::create_dir_all(&nested).unwrap();
        let _cwd = SaveCwd::chdir(&nested);
        let cfg = ConnectionConfig::from_env();
        assert_eq!(cfg.database, "frozen_catalog_abc1");
    }

    #[test]
    fn from_env_sets_coherence_project_slug_from_manifest_when_unset() {
        let _lock = LOCK.lock().unwrap();
        let _restore = MultiEnvRestore::snapshot(&[
            "DOLT_DB",
            "COHERENCE_PROJECT_SLUG",
            "COHERENCE_USE_USER_SCOPED_DOLT",
        ]);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::remove_var("COHERENCE_USE_USER_SCOPED_DOLT");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "dolt_db_name = \"x\"\n");
        let _cwd = SaveCwd::chdir(tmp.path());
        let _cfg = ConnectionConfig::from_env();
        assert_eq!(env::var("COHERENCE_PROJECT_SLUG").unwrap(), "myproj");
    }

    #[test]
    fn preflight_migrate_errors_when_user_scoped_and_manifest_without_dolt_db_name() {
        let _lock = LOCK.lock().unwrap();
        let _restore = MultiEnvRestore::snapshot(&[
            "DOLT_DB",
            "COHERENCE_PROJECT_SLUG",
            "COHERENCE_USE_USER_SCOPED_DOLT",
        ]);
        env::remove_var("DOLT_DB");
        env::remove_var("COHERENCE_PROJECT_SLUG");
        env::set_var("COHERENCE_USE_USER_SCOPED_DOLT", "1");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "");
        let _cwd = SaveCwd::chdir(tmp.path());
        let err = preflight_user_scoped_migrate_binding().unwrap_err();
        assert!(
            err.contains("dolt_db_name") && err.contains("project init"),
            "{err}"
        );
    }

    #[test]
    fn preflight_migrate_ok_when_dolt_db_explicit_under_user_scoped() {
        let _lock = LOCK.lock().unwrap();
        let _restore = MultiEnvRestore::snapshot(&[
            "DOLT_DB",
            "COHERENCE_PROJECT_SLUG",
            "COHERENCE_USE_USER_SCOPED_DOLT",
        ]);
        env::set_var("COHERENCE_USE_USER_SCOPED_DOLT", "1");
        env::set_var("DOLT_DB", "explicit_db");

        let tmp = tmp_git_repo();
        write_manifest(tmp.path(), "");
        let _cwd = SaveCwd::chdir(tmp.path());
        preflight_user_scoped_migrate_binding().unwrap();
    }
}
