//! Drops a disposable integration-test database on the user-scoped Dolt server (ADR-0004/0006).
//! Refuses non-test slugs: database name must start with `COHERENCE_TEST_DB_PREFIX` (default `coherence_test_`).

use mysql::prelude::Queryable;

use coherence_core_db::db::{self, ConnectionConfig, user_scoped_dolt_from_manifest};
use coherence_core_db::project_manifest;

fn configured_test_db_prefix() -> String {
    std::env::var("COHERENCE_TEST_DB_PREFIX").unwrap_or_else(|_| "coherence_test_".to_string())
}

pub fn run() -> i32 {
    let manifest = project_manifest::try_read_project_manifest_from_cwd();
    if !user_scoped_dolt_from_manifest(&manifest) {
        eprintln!(
            "drop-isolated-test-db: skipped (dolt_mode is not user-scoped in project.toml)"
        );
        return 0;
    }
    let config = match ConnectionConfig::from_env() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("drop-isolated-test-db: failed");
            eprintln!("{err}");
            return 1;
        }
    };
    let db_name = config.database.clone();
    let prefix = configured_test_db_prefix();
    if !db_name.starts_with(&prefix) {
        eprintln!(
            "drop-isolated-test-db: refused — database name must start with prefix {:?} (got {:?})",
            prefix, db_name
        );
        return 2;
    }
    let (mut conn, _) = match db::connect_without_database(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("drop-isolated-test-db: connect failed: {e}");
            return 1;
        }
    };
    let ident = db::mysql_quote_identifier(&db_name);
    let stmt = format!("DROP DATABASE IF EXISTS {ident}");
    conn.query_drop(stmt).map_or_else(
        |e| {
            eprintln!("drop-isolated-test-db: {e}");
            1
        },
        |_| {
            eprintln!("drop-isolated-test-db: dropped `{db_name}`");
            0
        },
    )
}
