use std::env;

use crate::project_manifest::{coherence_manifest_path, find_git_repo_root, read_manifest};

pub fn run() -> i32 {
    println!("coherence-core-db doctor");
    println!("status: ok");
    println!("workflow_backend: local_stub");
    println!("orchestration_owner: external");
    println!(
        "canonical_db_policy: curated catalog only for reasoning state — tests/smoke refuse writes unless COHERENCE_DB_PROFILE=test (see coherence-core-db help, AGENTS.md)"
    );
    println!(
        "managed_evidence: ADR-0005 per-run files under workspace .coherence/runs/<run-id>/ — demo: coherence-core-db evidence-sample"
    );
    print_project_manifest_diagnostic_lines();
    0
}

fn dolt_db_env_override_active() -> bool {
    env::var("DOLT_DB")
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

fn print_project_manifest_diagnostic_lines() {
    let env_override = dolt_db_env_override_active();

    match env::current_dir() {
        Ok(cwd) => match find_git_repo_root(cwd) {
            Some(root) => {
                println!("git_root_found: yes");
                let manifest_path = coherence_manifest_path(&root);
                println!("manifest_path: {}", manifest_path.display());
                if manifest_path.exists() {
                    println!("manifest_present: yes");
                    match read_manifest(&root) {
                        Ok(manifest) => {
                            if let Some(name) = manifest.dolt_db_name.as_ref() {
                                println!("dolt_db_name_manifest: {name}");
                            } else {
                                println!("dolt_db_name_manifest: missing");
                            }
                        }
                        Err(err) => {
                            println!("dolt_db_name_manifest: unreadable ({err})");
                        }
                    }
                } else {
                    println!("manifest_present: no");
                    println!("dolt_db_name_manifest: n/a");
                }
            }
            None => {
                println!("git_root_found: no");
                println!("manifest_path: n/a");
                println!("manifest_present: no");
                println!("dolt_db_name_manifest: n/a");
            }
        },
        Err(err) => {
            println!("git_root_found: unknown (cwd: {err})");
            println!("manifest_path: n/a");
            println!("manifest_present: unknown");
            println!("dolt_db_name_manifest: n/a");
        }
    }

    println!(
        "env_DOLT_DB_override_active: {}",
        if env_override { "yes" } else { "no" }
    );
}
