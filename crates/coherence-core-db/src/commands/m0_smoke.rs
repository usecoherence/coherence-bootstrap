use coherence_core_db::db::{
    connect, counts, insert_acceptance_criterion, insert_spec, ConnectionConfig,
};
use coherence_core_db::migrations;
use coherence_core_db::models::{AcceptanceCriterion, Spec};
use coherence_core_db::test_world_guard;

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
    let config = ConnectionConfig::from_env()?;
    test_world_guard::require_isolated_test_world_for_writes("m0-smoke", &config.database)?;

    println!("m0-smoke: run migrations");
    let applied = migrations::apply_all(&config)?;
    println!("migrations_applied: {applied}");

    println!("m0-smoke: connect to Dolt SQL");
    let (mut conn, mode) = connect(&config)?;
    println!("connection_mode: {mode}");
    println!("database: {}", config.database);

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
