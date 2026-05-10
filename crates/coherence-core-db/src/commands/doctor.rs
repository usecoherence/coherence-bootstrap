use std::env;

use crate::db::{
    explicit_dolt_database_from_env, hypothetical_effective_catalog_from_cwd,
    manifest_catalog_rules_without_dolt_db,
};
use crate::project_manifest::{
    coherence_env_from_std_env, coherence_manifest_path, find_git_repo_root, read_manifest,
};

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
    print_coherence_catalog_doctor_summary();
    print_project_manifest_diagnostic_lines();
    0
}

fn print_coherence_catalog_doctor_summary() {
    let env_override = explicit_dolt_database_from_env();

    match coherence_env_from_std_env() {
        Ok(te) => println!("COHERENCE_ENV: {}", te.as_str()),
        Err(e) => {
            println!("COHERENCE_ENV: invalid — {e} (migrate/db-ping will fail until corrected)")
        }
    }

    match hypothetical_effective_catalog_from_cwd() {
        Ok(name) => println!("effective_catalog_without_DOLT_DB_override: {name}"),
        Err(reason) => {
            println!("effective_catalog_without_DOLT_DB_override: unresolved ({reason})")
        }
    }

    if env_override {
        println!(
            "manifest_catalog_complete_for_connect_preflight: skipped (env DOLT_DB override active)"
        );
    } else {
        match manifest_catalog_rules_without_dolt_db() {
            Ok(()) => println!("manifest_catalog_complete_for_connect_preflight: yes"),
            Err(reason) => {
                println!("manifest_catalog_complete_for_connect_preflight: no ({reason})",)
            }
        }
    }

    println!(
        "env_DOLT_DB_override_active: {}",
        if env_override { "yes" } else { "no" }
    );
}

fn print_project_manifest_diagnostic_lines() {
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
                            if manifest
                                .project_hash
                                .as_ref()
                                .is_some_and(|h| !h.trim().is_empty())
                            {
                                println!("project_hash_manifest: yes");
                            } else {
                                println!("project_hash_manifest: missing");
                            }
                        }
                        Err(err) => {
                            println!("dolt_db_name_manifest: unreadable ({err})");
                            println!("project_hash_manifest: unreadable ({err})");
                        }
                    }
                } else {
                    println!("manifest_present: no");
                    println!("dolt_db_name_manifest: n/a");
                    println!("project_hash_manifest: n/a");
                }
            }
            None => {
                println!("git_root_found: no");
                println!("manifest_path: n/a");
                println!("manifest_present: no");
                println!("dolt_db_name_manifest: n/a");
                println!("project_hash_manifest: n/a");
            }
        },
        Err(err) => {
            println!("git_root_found: unknown (cwd: {err})");
            println!("manifest_path: n/a");
            println!("manifest_present: unknown");
            println!("dolt_db_name_manifest: n/a");
            println!("project_hash_manifest: n/a");
        }
    }
}
