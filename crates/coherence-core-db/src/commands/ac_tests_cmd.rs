//! `ac-tests` CLI: materialize Rust AC test skeleton files from the live spec graph.

use std::fs;
use std::path::{Path, PathBuf};

use crate::ac_test_layout::expected_rust_ac_test_files;
use crate::commands::cli_parse::parse_args;
use crate::db::{connect, ConnectionConfig};
use crate::migrations;
use crate::models::SpecGraph;
use crate::spec_store;

pub fn run(args: &[String]) -> i32 {
    match run_impl(args) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("ac-tests: {err}");
            1
        }
    }
}

fn run_impl(args: &[String]) -> Result<(), String> {
    let sub = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| "usage: coherence-core-db ac-tests <materialize-rust> ...".to_string())?;
    let tail = &args[1..];
    match sub {
        "materialize-rust" => materialize_rust(tail),
        other => Err(format!(
            "unknown ac-tests subcommand: {other} (expected materialize-rust)"
        )),
    }
}

fn connect_migrated() -> Result<mysql::Conn, String> {
    let config = ConnectionConfig::from_env();
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
    let mut expected = expected_rust_ac_test_files(graph);
    expected.sort_by(|a, b| a.file_path.cmp(&b.file_path));

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

fn materialize_rust(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let ws_flag = parsed.single_flag("workspace")?;
    let root = resolve_workspace_root(ws_flag)?;

    let mut conn = connect_migrated()?;
    let graph = spec_store::load_spec_graph(&mut conn)?;
    let (created, existing) = materialize_rust_ac_tests(&root, &graph)?;

    println!("Created:");
    for p in &created {
        println!("  {p}");
    }
    println!("Existing:");
    for p in &existing {
        println!("  {p}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AcceptanceCriterion, Spec, SpecGraph};

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
            "paths should be sorted: {:?}",
            created
        );
        assert!(created[0].contains("/ac-a."));
        assert!(created[1].contains("/ac-z."));
    }
}
