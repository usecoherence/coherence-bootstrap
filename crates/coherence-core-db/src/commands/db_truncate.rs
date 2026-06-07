//! Resets the catalog by deleting all spec/AC/codeintel rows.
//! Wires through the same connection path as the rest of the CLI (user-scoped socket).
//!
//! Usage: coherence-core-db truncate --env dev|test|prod --confirm

use mysql::prelude::Queryable;

use coherence_core_db::db::{
    self, mysql_quote_identifier, user_scoped_dolt_from_manifest, ConnectionConfig,
};
use coherence_core_db::project_manifest;

const VALID_ENVS: &[&str] = &["dev", "test", "prod"];

fn env_is_valid(env: &str) -> bool {
    VALID_ENVS.contains(&env)
}

pub fn run(args: &[String]) -> i32 {
    let mut args = args.iter();
    let mut target_env: Option<String> = None;
    let mut confirm = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--env" => {
                let val = args.next().map(String::as_str).unwrap_or_default();
                if !env_is_valid(val) {
                    eprintln!("truncate: --env must be one of dev|test|prod (got {val:?})");
                    return 1;
                }
                target_env = Some(val.to_string());
            }
            "--confirm" => {
                confirm = true;
            }
            other => {
                eprintln!("truncate: unknown flag: {other}");
                return 1;
            }
        }
    }

    let Some(target_env) = target_env else {
        eprintln!("truncate: --env dev|test|prod is required");
        return 1;
    };

    if !confirm {
        eprintln!("truncate --env {target_env}: --confirm is required to proceed");
        eprintln!("This will DELETE all rows from specs, acceptance_criteria, codeintel_ac_links,");
        eprintln!("codeintel_code_locations, spec_relations, acceptance_criterion_concerns.");
        return 1;
    }

    let manifest = project_manifest::try_read_project_manifest_from_cwd();
    if !user_scoped_dolt_from_manifest(&manifest) {
        eprintln!("truncate: skipped (dolt_mode is not user-scoped in project.toml)");
        return 0;
    }

    std::env::set_var("COHERENCE_ENV", &target_env);

    let config = match ConnectionConfig::from_env() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("truncate: ConnectionConfig::from_env() failed: {err}");
            return 1;
        }
    };

    let db_name = config.database.clone();
    eprintln!("truncate: target={db_name}");

    let (mut conn, _) = match db::connect_without_database(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("truncate: connect_without_database failed: {e}");
            return 1;
        }
    };

    let ident = mysql_quote_identifier(&db_name);
    if let Err(e) = conn.query_drop(format!("USE {ident}")) {
        eprintln!("truncate: USE {ident} failed: {e}");
        return 1;
    }

    let tables = [
        "acceptance_criterion_concerns",
        "codeintel_ac_links",
        "codeintel_code_locations",
        "spec_relations",
        "acceptance_criteria",
        "specs",
    ];

    for table in tables {
        let stmt = format!("DELETE FROM {table}");
        if let Err(e) = conn.query_drop(stmt.clone()) {
            eprintln!("truncate: {stmt} failed: {e}");
            return 1;
        }
        eprintln!("truncate: {table} — cleared");
    }

    let counts: Vec<(u64,)> = match conn.query("SELECT COUNT(*) FROM specs") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("truncate: verification query failed: {e}");
            return 1;
        }
    };
    let spec_count = counts.first().map_or(0, |(c,)| *c);

    eprintln!("truncate: done — {spec_count} specs remaining");
    0
}
