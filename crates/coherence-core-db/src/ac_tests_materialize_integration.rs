//! Isolated-DB integration: `ac-tests materialize-rust` seeds codeintel rows (COREDB-k34.3).

use std::path::{Path, PathBuf};
use std::process::Command;

use mysql::Conn;
use tempfile::tempdir;

use coherence_core_db::ac_code_link_store;
use coherence_core_db::ac_materialize_codeintel_ids::{
    ac_link_id_for_verified_by_file, code_location_id_for_materialized_ac_test,
};
use coherence_core_db::db::{self, ConnectionConfig};
use coherence_core_db::migrations;
use coherence_core_db::models::{AcCodeRelationKind, AcceptanceCriterion, CodeLocationKind, Spec};
use coherence_core_db::spec_store;
use coherence_core_db::test_world_guard;

fn coherence_core_db_test_bin() -> PathBuf {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_coherence-core-db") {
        return PathBuf::from(p);
    }
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    target.join("debug").join("coherence-core-db")
}

fn maybe_conn() -> Option<test_world_guard::EnvConnLock<Conn>> {
    let lock = test_world_guard::lock_test_env();
    let config = ConnectionConfig::from_env().ok()?;
    test_world_guard::panic_unless_isolated_test_world_for_writes(
        "ac_tests_materialize_integration",
        &config,
    );
    migrations::apply_all(&config).ok()?;
    let (conn, _) = db::connect(&config).ok()?;
    Some(test_world_guard::EnvConnLock {
        _lock: lock,
        inner: conn,
    })
}

#[test]
fn materialize_rust_cli_upserts_codeintel_rows() {
    let Some(mut conn) = maybe_conn() else {
        return;
    };

    let workspace = tempdir().expect("tempdir");
    let ws = workspace.path();

    let mut spec = Spec::new("SPEC-MAT-CLI", "materialize CLI spec");
    spec.description = "i".into();
    spec.created_at = "t1".into();
    spec.updated_at = "t1".into();
    spec_store::put_spec(&mut conn, &spec).expect("put_spec");

    let mut ac = AcceptanceCriterion::new("AC-MAT-CLI-1", "SPEC-MAT-CLI", "AC for materialize");
    ac.slug = "mat-cli-ac".into();
    ac.intent = "i".into();
    ac.created_at = "t1".into();
    ac.updated_at = "t1".into();
    spec_store::put_acceptance_criterion(&mut conn, &ac).expect("put ac");

    let graph = spec_store::load_spec_graph(&mut conn).expect("load graph");
    let files = coherence_core_db::ac_test_layout::expected_rust_ac_test_files(&graph);
    let ours = files
        .iter()
        .find(|f| f.ac_id == "AC-MAT-CLI-1")
        .expect("expected file entry for seeded AC");
    let rel_path = ours.file_path.clone();
    assert!(
        rel_path.starts_with("tests/ac/") && rel_path.ends_with(".rs"),
        "unexpected layout path {rel_path}"
    );

    let loc_id = code_location_id_for_materialized_ac_test("AC-MAT-CLI-1", ".", rel_path.as_str());

    let bin = coherence_core_db_test_bin();
    assert!(
        bin.is_file(),
        "coherence-core-db test binary missing at {} (build crate tests first)",
        bin.display()
    );
    let ws_str = ws.to_str().expect("utf8 workspace");
    let out = Command::new(&bin)
        .args(["ac-tests", "materialize-rust", "--workspace", ws_str])
        .output()
        .expect("spawn materialize-rust");
    assert!(
        out.status.success(),
        "materialize-rust failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let loc = ac_code_link_store::get_code_location(&mut conn, &loc_id)
        .expect("get_code_location")
        .expect("location row");
    assert_eq!(loc.repo_path, ".");
    assert_eq!(loc.file_path, rel_path);
    assert_eq!(loc.kind, CodeLocationKind::TestFile);
    assert_eq!(
        loc.test_command.as_deref(),
        Some(ours.test_command.as_str())
    );

    let link_id = ac_link_id_for_verified_by_file("AC-MAT-CLI-1", &loc_id);
    let links =
        ac_code_link_store::list_code_links_for_ac(&mut conn, "AC-MAT-CLI-1").expect("list links");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].link.id, link_id);
    assert_eq!(links[0].link.ac_id, "AC-MAT-CLI-1");
    assert_eq!(links[0].link.code_location_id, loc_id);
    assert_eq!(links[0].link.relation_kind, AcCodeRelationKind::VerifiedBy);
    assert_eq!(links[0].location.id, loc_id);
}
