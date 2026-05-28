//! `ac-tests` CLI: materialize and check Rust AC test files against the live spec graph.

use std::fs;
use std::path::{Path, PathBuf};

use coherence_core_db::ac_code_link_store;
use coherence_core_db::ac_materialize_codeintel_ids::{
    ac_link_id_for_verified_by_file, code_location_id_for_materialized_ac_test,
};
use coherence_core_db::ac_test_layout::{expected_rust_ac_test_files, ExpectedAcTestFile};
use crate::commands::cli_parse::parse_args;
use coherence_core_db::db::{connect, ConnectionConfig};
use coherence_core_db::migrations;
use coherence_core_db::models::{AcCodeLink, AcCodeRelationKind, CodeLocation, SpecGraph};
use coherence_core_db::spec_store;
use mysql::Conn;

pub fn run(args: &[String]) -> i32 {
    match run_impl(args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("ac-tests: {err}");
            1
        }
    }
}

fn run_impl(args: &[String]) -> Result<i32, String> {
    let sub = args.first().map(String::as_str).ok_or_else(|| {
        "usage: coherence-core-db ac-tests <materialize-rust | check-rust> [--workspace <dir>] ..."
            .to_string()
    })?;
    let tail = &args[1..];
    match sub {
        "materialize-rust" => {
            materialize_rust(tail)?;
            Ok(0)
        }
        "check-rust" => check_rust(tail),
        other => Err(format!(
            "unknown ac-tests subcommand: {other} (expected materialize-rust or check-rust)\n\
             materialize-rust: create missing tests/ac/**/*.rs from the DB graph (see ac-tests check-rust)\n\
             check-rust: verify every expected file exists; exits 1 if any are missing"
        )),
    }
}

fn connect_migrated() -> Result<mysql::Conn, String> {
    let config = ConnectionConfig::from_env()?;
    migrations::apply_all(&config)?;
    let (conn, _) = connect(&config)?;
    Ok(conn)
}

/// Default workspace: walk up from [`std::env::current_dir`] for a directory containing `AGENTS.md`
/// (this repo’s marker); if none, use the current directory. With `--workspace`, that directory
/// must exist and is canonicalized.
fn resolve_workspace_root(workspace_flag: Option<&str>) -> Result<PathBuf, String> {
    if let Some(p) = workspace_flag {
        let pb = PathBuf::from(p);
        let meta = fs::metadata(&pb).map_err(|e| format!("--workspace {p:?}: {e}"))?;
        if !meta.is_dir() {
            return Err(format!("--workspace {p:?} is not a directory"));
        }
        return pb.canonicalize().map_err(|e| format!("--workspace: {e}"));
    }

    let cwd =
        std::env::current_dir().map_err(|e| format!("could not read current directory: {e}"))?;
    let mut cur = cwd.clone();
    loop {
        if cur.join("AGENTS.md").is_file() {
            return cur
                .canonicalize()
                .map_err(|e| format!("workspace root: {e}"));
        }
        if !cur.pop() {
            return Ok(cwd);
        }
    }
}

/// Expected per-AC Rust files in deterministic order (`file_path`).
/// Shared by [`materialize_rust_ac_tests`] and [`missing_rust_ac_test_files`].
fn sorted_expected_rust_ac_test_files(graph: &SpecGraph) -> Vec<ExpectedAcTestFile> {
    let mut expected = expected_rust_ac_test_files(graph);
    expected.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    expected
}

fn validate_tests_ac_rel_path(rel: &str) -> Result<(), String> {
    if !rel.starts_with("tests/ac/") {
        return Err(format!(
            "internal layout error: expected path under tests/ac/, got {rel:?}"
        ));
    }
    if rel.contains("..") {
        return Err(format!("unsafe relative path: {rel:?}"));
    }
    Ok(())
}

/// Writes missing skeleton files under `workspace/tests/ac/`. Does not overwrite existing files.
/// Returns `(created_rel_paths, existing_rel_paths)` using the same relative strings as layout.
fn materialize_rust_ac_tests(
    workspace: &Path,
    graph: &SpecGraph,
) -> Result<(Vec<String>, Vec<String>), String> {
    let expected = sorted_expected_rust_ac_test_files(graph);

    let tests_ac_root = workspace.join("tests/ac");

    let mut created = Vec::new();
    let mut existing = Vec::new();

    for file in expected {
        validate_tests_ac_rel_path(&file.file_path)?;
        let abs = workspace.join(&file.file_path);
        if !abs.starts_with(&tests_ac_root) {
            return Err(format!(
                "refusing to write outside tests/ac: {}",
                abs.display()
            ));
        }

        if abs.is_file() {
            existing.push(file.file_path);
            continue;
        }
        if abs.exists() {
            return Err(format!(
                "refusing to write: {} exists and is not a regular file",
                abs.display()
            ));
        }
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("create_dir_all {}: {e}", parent.display()))?;
        }
        fs::write(&abs, file.content.as_bytes())
            .map_err(|e| format!("write {}: {e}", abs.display()))?;
        created.push(file.file_path);
    }

    Ok((created, existing))
}

/// After filesystem materialization, upsert `codeintel_code_locations` + `codeintel_ac_links`
/// (`verified_by`) for every **expected** AC Rust test file that exists on disk under `workspace`.
///
/// Product choice (COREDB-k34.2): reconcile links for all present expected files each run (not only
/// files created in the just-finished pass) so repeated `materialize-rust` keeps the DB aligned with
/// `tests/ac/**` and `verify-ac` remains consistent.
fn upsert_codeintel_for_expected_ac_test_files(
    conn: &mut Conn,
    workspace: &Path,
    graph: &SpecGraph,
) -> Result<usize, String> {
    let mut n = 0usize;
    for file in sorted_expected_rust_ac_test_files(graph) {
        validate_tests_ac_rel_path(&file.file_path)?;
        let abs = workspace.join(&file.file_path);
        if !abs.is_file() {
            continue;
        }

        let loc_id = code_location_id_for_materialized_ac_test(
            file.ac_id.as_str(),
            ".",
            file.file_path.as_str(),
        );
        let mut loc = CodeLocation::new(loc_id.clone(), ".", file.file_path.as_str());
        loc.test_command = Some(file.test_command.clone());
        ac_code_link_store::put_code_location(conn, &loc)?;

        let link_id = ac_link_id_for_verified_by_file(file.ac_id.as_str(), &loc_id);
        let link = AcCodeLink::new(
            link_id,
            file.ac_id.as_str(),
            loc_id.as_str(),
            AcCodeRelationKind::VerifiedBy,
        );
        ac_code_link_store::put_ac_code_link(conn, &link)?;
        n += 1;
    }
    Ok(n)
}

/// Lists `(ac_id, relative file_path)` for expected Rust AC tests that are absent or not regular files.
fn missing_rust_ac_test_files(
    workspace: &Path,
    graph: &SpecGraph,
) -> Result<Vec<(String, String)>, String> {
    let tests_ac_root = workspace.join("tests/ac");
    let mut missing = Vec::new();

    for file in sorted_expected_rust_ac_test_files(graph) {
        validate_tests_ac_rel_path(&file.file_path)?;
        let abs = workspace.join(&file.file_path);
        if !abs.starts_with(&tests_ac_root) {
            return Err(format!(
                "refusing to scan outside tests/ac: {}",
                abs.display()
            ));
        }
        if abs.exists() && !abs.is_file() {
            return Err(format!(
                "expected AC test path exists but is not a regular file: {}",
                abs.display()
            ));
        }
        if !abs.is_file() {
            missing.push((file.ac_id, file.file_path));
        }
    }

    Ok(missing)
}

fn materialize_rust(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let ws_flag = parsed.single_flag("workspace")?;
    let root = resolve_workspace_root(ws_flag)?;

    let mut conn = connect_migrated()?;
    let graph = spec_store::load_spec_graph(&mut conn)?;
    let (created, existing) = materialize_rust_ac_tests(&root, &graph)?;
    let codeintel_n = upsert_codeintel_for_expected_ac_test_files(&mut conn, &root, &graph)?;

    println!("Created:");
    for p in &created {
        println!("  {p}");
    }
    println!("Existing:");
    for p in &existing {
        println!("  {p}");
    }
    println!("codeintel: upserted {codeintel_n} verified_by test file link(s)");
    Ok(())
}

fn check_rust(args: &[String]) -> Result<i32, String> {
    let parsed = parse_args(args)?;
    let ws_flag = parsed.single_flag("workspace")?;
    let root = resolve_workspace_root(ws_flag)?;

    let mut conn = connect_migrated()?;
    let graph = spec_store::load_spec_graph(&mut conn)?;
    let missing = missing_rust_ac_test_files(&root, &graph)?;

    if missing.is_empty() {
        println!("All expected AC test files are present.");
        Ok(0)
    } else {
        println!("Missing AC test files:");
        for (ac_id, path) in &missing {
            println!("  {ac_id} -> {path}");
        }
        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use coherence_core_db::models::{AcceptanceCriterion, Spec, SpecGraph};

    #[test]
    fn validate_tests_ac_rel_path_rejects_traversal() {
        assert!(validate_tests_ac_rel_path("tests/ac/../etc/passwd").is_err());
    }

    #[test]
    fn materialize_creates_then_existing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();

        let mut ac = AcceptanceCriterion::new("AC-MAT-1", "SPEC-R", "Testing materialize");
        ac.slug = "sample-ac".into();
        let spec = Spec::new("SPEC-R", "Root");
        let graph = SpecGraph::new(vec![spec], vec![ac], vec![]);

        let (c1, e1) = materialize_rust_ac_tests(root, &graph).expect("first run");
        assert_eq!(c1.len(), 1);
        assert!(c1[0].ends_with("sample-ac.rs"));
        assert!(e1.is_empty());

        let (c2, e2) = materialize_rust_ac_tests(root, &graph).expect("second run");
        assert!(c2.is_empty());
        assert_eq!(e2, c1);

        let path = root.join(&c1[0]);
        let body = fs::read_to_string(&path).expect("read");
        assert!(body.contains("AC-MAT-1"));
        assert!(body.contains("fn validates_sample_ac()"));
    }

    #[test]
    fn materialize_sorts_output_paths_by_file_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let spec = Spec::new("SPEC-R", "Root");

        let ac_z = AcceptanceCriterion::new("AC-Z", "SPEC-R", "z");
        let ac_a = AcceptanceCriterion::new("AC-A", "SPEC-R", "a");
        let graph = SpecGraph::new(vec![spec], vec![ac_z, ac_a], vec![]);

        let (created, _) = materialize_rust_ac_tests(root, &graph).expect("materialize");
        assert_eq!(created.len(), 2);
        assert!(
            created[0] < created[1],
            "paths should be sorted: {created:?}",
        );
        assert!(created[0].contains("/ac-a."));
        assert!(created[1].contains("/ac-z."));
    }

    #[test]
    fn check_reports_none_missing_after_materialize() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();

        let mut ac = AcceptanceCriterion::new("AC-CHK-1", "SPEC-R", "Check happy path");
        ac.slug = "chk-ac".into();
        let spec = Spec::new("SPEC-R", "Root");
        let graph = SpecGraph::new(vec![spec], vec![ac], vec![]);

        materialize_rust_ac_tests(root, &graph).expect("materialize");
        let missing = missing_rust_ac_test_files(root, &graph).expect("check scan");
        assert!(missing.is_empty());
    }

    #[test]
    fn check_reports_missing_after_file_deleted() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();

        let mut ac = AcceptanceCriterion::new("AC-DEL", "SPEC-R", "Del");
        ac.slug = "gone-ac".into();
        let spec = Spec::new("SPEC-R", "Root");
        let graph = SpecGraph::new(vec![spec], vec![ac], vec![]);

        let (created, _) = materialize_rust_ac_tests(root, &graph).expect("materialize");
        fs::remove_file(root.join(&created[0])).expect("remove");
        let missing = missing_rust_ac_test_files(root, &graph).expect("check scan");
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].0, "AC-DEL");
        assert_eq!(missing[0].1, created[0]);
    }

    #[test]
    fn check_reports_missing_when_graph_extended_without_materialize() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();

        let mut ac1 = AcceptanceCriterion::new("AC-ONE", "SPEC-R", "One");
        ac1.slug = "one-ac".into();
        let spec = Spec::new("SPEC-R", "Root");
        let graph1 = SpecGraph::new(vec![spec.clone()], vec![ac1.clone()], vec![]);
        materialize_rust_ac_tests(root, &graph1).expect("materialize");

        let ac2 = AcceptanceCriterion::new("AC-TWO", "SPEC-R", "Two");
        let graph2 = SpecGraph::new(vec![spec], vec![ac1, ac2], vec![]);
        let missing = missing_rust_ac_test_files(root, &graph2).expect("check scan");
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].0, "AC-TWO");
    }
}
