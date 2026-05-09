use std::path::PathBuf;

use crate::db::{
    apply_schema_from_file, connect, counts, insert_acceptance_criterion, insert_spec,
    ConnectionConfig,
};
use crate::models::{AcceptanceCriterion, Spec};

pub fn run() -> i32 {
    match run_impl() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("m0-smoke: failed");
            eprintln!("{err}");
            1
        }
    }
}

fn run_impl() -> Result<(), String> {
    let config = ConnectionConfig::from_env();
    let schema_path = default_schema_path();

    println!("m0-smoke: connect to Dolt SQL");
    let (mut conn, mode) = connect(&config)?;
    println!("connection_mode: {mode}");
    println!("database: {}", config.database);

    println!("m0-smoke: apply schema from {}", schema_path.display());
    apply_schema_from_file(&mut conn, &schema_path)?;

    let spec = Spec::new("SPEC-1", "Minimal core-db smoke spec");
    let ac = AcceptanceCriterion::new(
        "AC-1",
        "SPEC-1",
        "Minimal core-db smoke acceptance criterion",
    );

    insert_spec(&mut conn, &spec)?;
    insert_acceptance_criterion(&mut conn, &ac)?;

    let (spec_count, ac_count) = counts(&mut conn)?;
    println!("counts: specs={spec_count}, acceptance_criteria={ac_count}");
    println!("m0-smoke: success");
    Ok(())
}

fn default_schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sql/m0_schema.sql")
}
