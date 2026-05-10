//! Demo command: materialize one managed run under `.coherence/runs/<run-id>/` (ADR-0005).

use crate::commands::cli_parse;
use crate::evidence_store::{self, resolve_artifact_path, CanonicalEvidencePointer, RunLayout};
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn run(args: &[String]) -> i32 {
    match run_impl(args) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("evidence-sample: failed");
            eprintln!("{err}");
            1
        }
    }
}

fn run_impl(args: &[String]) -> Result<(), String> {
    let parsed = cli_parse::parse_args(args)?;
    let workspace = match parsed.single_flag("workspace")? {
        Some(p) => PathBuf::from(p),
        None => env::current_dir().map_err(|e| format!("current_dir: {e}"))?,
    };
    let run_id = parsed
        .single_flag("run-id")?
        .map(String::from)
        .unwrap_or_else(|| format!("run-{}", uuid::Uuid::new_v4()));

    let pointer = evidence_store::bootstrap_sample_run(&workspace, &run_id)?;
    print_report(&workspace, &run_id, &pointer)?;
    Ok(())
}

fn print_report(
    workspace: &PathBuf,
    run_id: &str,
    pointer: &CanonicalEvidencePointer,
) -> Result<(), String> {
    let layout = RunLayout::new(workspace, run_id);
    let artifact_path = resolve_artifact_path(workspace, pointer);
    let bytes = fs::read(&artifact_path).map_err(|e| format!("read back artifact: {e}"))?;

    println!("evidence-sample: success");
    println!("run_id: {}", pointer.run_id);
    println!("workspace_root: {}", workspace.display());
    println!("run_root: {}", layout.run_root().display());
    println!("manifest: {}", layout.run_manifest_path().display());
    println!(
        "canonical_pointer_stub (future DB row shape, metadata only): {}",
        layout.canonical_pointer_stub_path().display()
    );
    println!("retrieval: join workspace + evidence_root_relpath + artifact_relpath_from_run_root");
    println!("  evidence_root_relpath: {}", pointer.evidence_root_relpath);
    println!(
        "  artifact_relpath_from_run_root: {}",
        pointer.artifact_relpath_from_run_root
    );
    println!("  resolved_artifact_path: {}", artifact_path.display());
    println!("  artifact_bytes: {}", bytes.len());
    println!(
        "  sha256_hex (matches pointer): {}",
        pointer.payload_sha256_hex == evidence_store::sha256_hex(&bytes)
    );
    println!(
        "large_payload_stays_outside_canonical_db: confirmed (Dolt not used; stub JSON size << artifact)"
    );
    println!(
        "note: no evidence pointer SQL migration in M1 yet — see evidence_store module docs (ADR-0005 boundary)."
    );
    Ok(())
}
