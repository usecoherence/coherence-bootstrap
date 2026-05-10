use std::collections::HashSet;

use crate::db::{connect, ConnectionConfig};
use crate::migrations;
use crate::models::{AcceptanceCriterion, Spec, SpecRelation};
use crate::spec_store;
use crate::test_world_guard;

/// Smoke identifiers — stable and namespaced away from `m0-smoke` fixtures.
const SMOKE_SPEC_ID: &str = "M1-SMOKE-SPEC-1";
const SMOKE_AC_ID: &str = "M1-SMOKE-AC-1";
const SMOKE_REL_ID: &str = "M1-SMOKE-REL-1";

pub fn run() -> i32 {
    match run_impl() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("m1-spec-smoke: failed");
            eprintln!("{err}");
            1
        }
    }
}

fn count_acceptance_criteria(conn: &mut mysql::Conn) -> Result<usize, String> {
    let specs = spec_store::list_specs(conn)?;
    let mut n = 0;
    for spec in specs {
        n += spec_store::list_acceptance_criteria_for_spec(conn, &spec.id)?.len();
    }
    Ok(n)
}

fn count_spec_relations(conn: &mut mysql::Conn) -> Result<usize, String> {
    let specs = spec_store::list_specs(conn)?;
    let mut seen = HashSet::new();
    for spec in specs {
        for rel in spec_store::list_spec_relations_for_spec(conn, &spec.id)? {
            seen.insert(rel.id);
        }
    }
    Ok(seen.len())
}

fn run_impl() -> Result<(), String> {
    let config = ConnectionConfig::from_env();
    test_world_guard::require_isolated_test_world_for_writes("m1-spec-smoke", &config)?;

    println!("m1-spec-smoke: run migrations");
    let applied = migrations::apply_all(&config)?;
    println!("migrations_applied: {applied}");

    println!("m1-spec-smoke: connect to Dolt SQL");
    let (mut conn, mode) = connect(&config)?;
    println!("connection_mode: {mode}");
    println!("database: {}", config.database);

    let mut spec = Spec::new(SMOKE_SPEC_ID, "M1 spec store smoke fixture");
    spec.description = "m1-spec-smoke persistence check".to_string();
    spec.created_at = "m1-smoke".to_string();
    spec.updated_at = "m1-smoke".to_string();

    let mut ac = AcceptanceCriterion::new(SMOKE_AC_ID, SMOKE_SPEC_ID, "M1 smoke AC");
    ac.intent = "verify put/list/read for acceptance_criteria".to_string();
    ac.created_at = "m1-smoke".to_string();
    ac.updated_at = "m1-smoke".to_string();

    // Self-loop relation: schema has no FK; one Spec still exercises spec_relations.
    let relation = SpecRelation::new(
        SMOKE_REL_ID,
        SMOKE_SPEC_ID,
        SMOKE_SPEC_ID,
        "smoke_example",
        "m1-spec-smoke relation round-trip",
    );

    spec_store::put_spec(&mut conn, &spec)?;
    spec_store::put_acceptance_criterion(&mut conn, &ac)?;
    spec_store::put_spec_relation(&mut conn, &relation)?;

    let loaded_spec = spec_store::get_spec(&mut conn, SMOKE_SPEC_ID)?
        .ok_or_else(|| format!("expected spec {SMOKE_SPEC_ID} after put"))?;
    if loaded_spec.title != spec.title {
        return Err(format!(
            "spec title mismatch: got {:?}, want {:?}",
            loaded_spec.title, spec.title
        ));
    }

    let acs = spec_store::list_acceptance_criteria_for_spec(&mut conn, SMOKE_SPEC_ID)?;
    if !acs.iter().any(|row| row.id == SMOKE_AC_ID) {
        return Err("acceptance criterion not found after put".into());
    }

    let rels = spec_store::list_spec_relations_for_spec(&mut conn, SMOKE_SPEC_ID)?;
    if !rels.iter().any(|row| row.id == SMOKE_REL_ID) {
        return Err("spec relation not found after put".into());
    }

    let spec_count = spec_store::list_specs(&mut conn)?.len();
    let ac_count = count_acceptance_criteria(&mut conn)?;
    let relation_count = count_spec_relations(&mut conn)?;

    println!("spec_count: {spec_count}");
    println!("ac_count: {ac_count}");
    println!("relation_count: {relation_count}");
    println!("m1-spec-smoke: success");
    Ok(())
}
