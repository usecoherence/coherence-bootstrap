use std::env;
use std::path::PathBuf;

use mysql::prelude::Queryable;

use crate::db::{self, ConnectionConfig};

fn resolve_config() -> ConnectionConfig {
    let socket_path = env::var("DOLT_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| db::user_scoped_socket_default_path());
    let host = env::var("DOLT_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let user = env::var("DOLT_USER").unwrap_or_else(|_| "root".to_string());
    let password = env::var("DOLT_PASSWORD").ok();
    let port = env::var("DOLT_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3306);
    ConnectionConfig {
        socket_path,
        host,
        port,
        user,
        password,
        database: String::new(),
    }
}

pub fn run() -> i32 {
    let config = resolve_config();
    let (mut conn, mode) = match db::connect_without_database(&config) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("db-list-databases: failed");
            eprintln!("{err}");
            return 1;
        }
    };
    let rows: Vec<String> = match conn.query("SHOW DATABASES") {
        Ok(rows) => rows,
        Err(err) => {
            eprintln!("db-list-databases: query failed: {err}");
            return 1;
        }
    };
    println!("Databases on {} ({}):", config.socket_path.display(), mode);
    for db_name in rows {
        if !db_name.is_empty() {
            println!("  {db_name}");
        }
    }
    0
}
