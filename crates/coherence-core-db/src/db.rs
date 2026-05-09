use std::env;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use mysql::prelude::Queryable;
use mysql::{params, Conn, OptsBuilder};

use crate::models::{AcceptanceCriterion, Spec};

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

impl ConnectionConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let socket_path = env::var("DOLT_SOCKET")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".dolt/dolt.sock"));
        let host = env::var("DOLT_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("DOLT_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(3306);
        let user = env::var("DOLT_USER").unwrap_or_else(|_| "root".to_string());
        let password = env::var("DOLT_PASSWORD").ok();
        let database = env::var("DOLT_DB").unwrap_or_else(|_| default_database_name());
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

fn default_database_name() -> String {
    env::current_dir()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().replace('-', "_"))
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

pub fn apply_schema_from_file(conn: &mut Conn, schema_path: &Path) -> Result<(), String> {
    let sql = fs::read_to_string(schema_path).map_err(|err| {
        format!(
            "failed to read schema file {}: {err}",
            schema_path.display()
        )
    })?;
    apply_schema_sql(conn, &sql)
}

fn apply_schema_sql(conn: &mut Conn, sql: &str) -> Result<(), String> {
    for statement in sql.split(';') {
        let trimmed = statement.trim();
        if trimmed.is_empty() {
            continue;
        }
        let executable = format!("{trimmed};");
        conn.query_drop(executable)
            .map_err(|err| format!("failed to execute schema statement: {err}"))?;
    }
    Ok(())
}

pub fn insert_spec(conn: &mut Conn, spec: &Spec) -> Result<(), String> {
    conn.exec_drop(
        r"INSERT INTO specs (id, title) VALUES (:id, :title)
          ON DUPLICATE KEY UPDATE title = VALUES(title)",
        params! {
            "id" => spec.id.as_str(),
            "title" => spec.title.as_str(),
        },
    )
    .map_err(|err| format!("failed to insert spec {}: {err}", spec.id))
}

pub fn insert_acceptance_criterion(
    conn: &mut Conn,
    ac: &AcceptanceCriterion,
) -> Result<(), String> {
    conn.exec_drop(
        r"INSERT INTO acceptance_criteria (id, spec_id, title) VALUES (:id, :spec_id, :title)
          ON DUPLICATE KEY UPDATE spec_id = VALUES(spec_id), title = VALUES(title)",
        params! {
            "id" => ac.id.as_str(),
            "spec_id" => ac.spec_id.as_str(),
            "title" => ac.title.as_str(),
        },
    )
    .map_err(|err| format!("failed to insert acceptance criterion {}: {err}", ac.id))
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
