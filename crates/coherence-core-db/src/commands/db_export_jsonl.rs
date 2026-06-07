//! Exports all semantic catalog tables to JSONL.
//! Usage: `coherence-core-db db export-jsonl --env dev [--out <file>]`
//!
//! Exports: `specs`, `acceptance_criteria`, `spec_relations`,
//!          `acceptance_criterion_concerns`, `codeintel_code_locations`, `codeintel_ac_links`

#![allow(clippy::too_many_lines, clippy::expect_used)]

use crate::db::{
    connect_without_database, mysql_quote_identifier, user_scoped_dolt_from_manifest,
    ConnectionConfig,
};
use crate::project_manifest;
use mysql::prelude::Queryable;
use std::io::Write;

pub fn run(args: &[String]) -> i32 {
    let mut args = args.iter();
    let mut target_env: Option<String> = None;
    let mut out_path: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--env" => {
                target_env = Some(
                    args.next()
                        .map(String::as_str)
                        .unwrap_or_default()
                        .to_string(),
                )
            }
            "--out" => {
                out_path = Some(
                    args.next()
                        .map(String::as_str)
                        .unwrap_or_default()
                        .to_string(),
                )
            }
            other => {
                eprintln!("export-jsonl: unknown flag: {other}");
                return 1;
            }
        }
    }

    let target_env = match target_env {
        Some(e) if e == "dev" || e == "test" || e == "prod" => e,
        Some(e) => {
            eprintln!("export-jsonl: --env must be dev|test|prod (got {e})");
            return 1;
        }
        None => {
            eprintln!("export-jsonl: --env dev|test|prod is required");
            return 1;
        }
    };

    let out_path =
        out_path.unwrap_or_else(|| ".coherence/exports/bootstrap-specs.jsonl".to_string());

    let manifest = project_manifest::try_read_project_manifest_from_cwd();
    if !user_scoped_dolt_from_manifest(&manifest) {
        eprintln!("export-jsonl: skipped (dolt_mode is not user-scoped)");
        return 0;
    }

    std::env::set_var("COHERENCE_ENV", &target_env);
    let config = match ConnectionConfig::from_env() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("export-jsonl: ConnectionConfig::from_env() failed: {err}");
            return 1;
        }
    };

    let (mut conn, _) = match connect_without_database(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("export-jsonl: connect failed: {e}");
            return 1;
        }
    };

    let ident = mysql_quote_identifier(&config.database);
    if let Err(e) = conn.query_drop(format!("USE {ident}")) {
        eprintln!("export-jsonl: USE {ident} failed: {e}");
        return 1;
    }

    if let Some(parent) = std::path::Path::new(&out_path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    let mut file = match std::fs::File::create(&out_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("export-jsonl: cannot create {out_path}: {e}");
            return 1;
        }
    };

    let mut count = 0usize;

    // 1. specs
    let specs: Vec<(String, String, String, String, String, String, String, String)> = conn.query(
        "SELECT id, slug, level, status, title, description, created_at, updated_at FROM specs ORDER BY id"
    ).expect("export: query failed");

    for (id, slug, level, status, title, description, created_at, updated_at) in specs {
        let rec = serde_json::json!({
            "type": "spec",
            "id": id,
            "slug": slug,
            "level": level,
            "status": status,
            "title": title,
            "description": description,
            "created_at": created_at,
            "updated_at": updated_at
        });
        let _ = writeln!(file, "{rec}");
        count += 1;
    }

    // 2. acceptance_criteria
    let acs: Vec<(String, String, String, String, String, String, String, String, String)> = conn.query(
        "SELECT id, spec_id, slug, title, intent, review_mode, risk_level, created_at, updated_at FROM acceptance_criteria ORDER BY id"
    ).expect("export: query failed");

    for (id, spec_id, slug, title, intent, review_mode, risk_level, created_at, updated_at) in acs {
        let rec = serde_json::json!({
            "type": "acceptance_criterion",
            "id": id,
            "spec_id": spec_id,
            "slug": slug,
            "title": title,
            "intent": intent,
            "review_mode": review_mode,
            "risk_level": risk_level,
            "created_at": created_at,
            "updated_at": updated_at
        });
        let _ = writeln!(file, "{rec}");
        count += 1;
    }

    // 3. spec_relations
    let rels: Vec<(String, String, String, String, String, String)> = conn.query(
        "SELECT id, source_spec_id, target_spec_id, relation_kind, created_at, updated_at FROM spec_relations ORDER BY 1"
    ).expect("export: query failed");

    for (id, source_spec_id, target_spec_id, relation_kind, created_at, updated_at) in rels {
        let rec = serde_json::json!({
            "type": "spec_relation",
            "id": id,
            "source_spec_id": source_spec_id,
            "target_spec_id": target_spec_id,
            "relation_kind": relation_kind,
            "created_at": created_at,
            "updated_at": updated_at
        });
        let _ = writeln!(file, "{rec}");
        count += 1;
    }

    // 4. acceptance_criterion_concerns
    let concerns: Vec<(String, String)> = conn
        .query("SELECT ac_id, concern_kind FROM acceptance_criterion_concerns ORDER BY 1")
        .expect("export: query failed");

    for (ac_id, concern_kind) in concerns {
        let rec = serde_json::json!({
            "type": "acceptance_criterion_concern",
            "ac_id": ac_id,
            "concern_kind": concern_kind
        });
        let _ = writeln!(file, "{rec}");
        count += 1;
    }

    // 5. codeintel_code_locations
    let locations: Vec<(String, String, String, String, String, String, String, String)> = conn.query(
        "SELECT id, repo_path, file_path, kind, symbol, test_command, created_at, updated_at FROM codeintel_code_locations ORDER BY id"
    ).expect("export: query failed");

    for (id, repo_path, file_path, kind, symbol, test_command, created_at, updated_at) in locations
    {
        let rec = serde_json::json!({
            "type": "codeintel_code_location",
            "id": id,
            "repo_path": repo_path,
            "file_path": file_path,
            "kind": kind,
            "symbol": symbol,
            "test_command": test_command,
            "created_at": created_at,
            "updated_at": updated_at
        });
        let _ = writeln!(file, "{rec}");
        count += 1;
    }

    // 6. codeintel_ac_links
    let links: Vec<(String, String, String, String, String, String, String)> = conn.query(
        "SELECT id, ac_id, code_location_id, relation_kind, note, created_at, updated_at FROM codeintel_ac_links ORDER BY id"
    ).expect("export: query failed");

    for (id, ac_id, code_location_id, relation_kind, note, created_at, updated_at) in links {
        let rec = serde_json::json!({
            "type": "codeintel_ac_link",
            "id": id,
            "ac_id": ac_id,
            "code_location_id": code_location_id,
            "relation_kind": relation_kind,
            "note": note,
            "created_at": created_at,
            "updated_at": updated_at
        });
        let _ = writeln!(file, "{rec}");
        count += 1;
    }

    eprintln!("export-jsonl: exported {count} records to {out_path}");
    0
}
