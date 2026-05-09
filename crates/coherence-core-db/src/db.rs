use std::env;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use mysql::prelude::Queryable;
use mysql::{Conn, OptsBuilder};

use crate::models::{AcceptanceCriterion, Spec};
use crate::spec_store;

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
