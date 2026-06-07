//! Imports all semantic catalog tables from JSONL.
//! Usage: `coherence-core-db db import-jsonl --env prod --in <file> --confirm`
//!
//! Imports: `specs`, `acceptance_criteria`, `spec_relations`,
//!          `acceptance_criterion_concerns`, `codeintel_code_locations`, `codeintel_ac_links`

#![allow(clippy::too_many_lines)]

use std::io::BufRead;
use mysql::prelude::Queryable;
use crate::db::{connect_without_database, user_scoped_dolt_from_manifest, mysql_quote_identifier, ConnectionConfig};
use crate::project_manifest;

pub fn run(args: &[String]) -> i32 {
    let mut args = args.iter();
    let mut target_env: Option<String> = None;
    let mut in_path: Option<String> = None;
    let mut confirm = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--env" => target_env = Some(args.next().map(String::as_str).unwrap_or_default().to_string()),
            "--in" => in_path = Some(args.next().map(String::as_str).unwrap_or_default().to_string()),
            "--confirm" => confirm = true,
            other => {
                eprintln!("import-jsonl: unknown flag: {other}");
                return 1;
            }
        }
    }

    let target_env = match target_env {
        Some(e) if e == "dev" || e == "test" || e == "prod" => e,
        Some(e) => {
            eprintln!("import-jsonl: --env must be dev|test|prod (got {e})");
            return 1;
        }
        None => {
            eprintln!("import-jsonl: --env dev|test|prod is required");
            return 1;
        }
    };

    let Some(in_path) = in_path else {
        eprintln!("import-jsonl: --in <file> is required");
        return 1;
    };

    if !confirm {
        eprintln!("import-jsonl --env {target_env} --in {in_path}: --confirm is required");
        return 1;
    }

    let manifest = project_manifest::try_read_project_manifest_from_cwd();
    if !user_scoped_dolt_from_manifest(&manifest) {
        eprintln!("import-jsonl: skipped (dolt_mode is not user-scoped)");
        return 0;
    }

    std::env::set_var("COHERENCE_ENV", &target_env);
    let config = match ConnectionConfig::from_env() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("import-jsonl: ConnectionConfig::from_env() failed: {err}");
            return 1;
        }
    };

    let (mut conn, _) = match connect_without_database(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("import-jsonl: connect failed: {e}");
            return 1;
        }
    };

    let ident = mysql_quote_identifier(&config.database);
    if let Err(e) = conn.query_drop(format!("USE {ident}")) {
        eprintln!("import-jsonl: USE {ident} failed: {e}");
        return 1;
    }

    let file = match std::fs::File::open(&in_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("import-jsonl: cannot open {in_path}: {e}");
            return 1;
        }
    };

    let reader = std::io::BufReader::new(file);
    let mut specs = 0usize;
    let mut acs = 0usize;
    let mut rels = 0usize;
    let mut concerns = 0usize;
    let mut locations = 0usize;
    let mut links = 0usize;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("import-jsonl: read error: {e}");
                return 1;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let value: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("import-jsonl: parse error: {e}");
                return 1;
            }
        };

        let rec_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match rec_type {
            "spec" => {
                let id = json_str(&value["id"]);
                let slug = json_str(&value["slug"]);
                let level = json_str(&value["level"]);
                let status = json_str(&value["status"]);
                let title = json_str(&value["title"]);
                let description = json_str(&value["description"]);
                let created_at = json_str(&value["created_at"]);
                let updated_at = json_str(&value["updated_at"]);

                let q = format!(
                    "INSERT INTO specs (id, slug, level, status, title, description, created_at, updated_at) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}')",
                    escape(&id), escape(&slug), escape(&level), escape(&status), escape(&title), escape(&description), escape(&created_at), escape(&updated_at)
                );
                if let Err(e) = conn.query_drop(&q) {
                    eprintln!("import-jsonl: INSERT spec {id} failed: {e}");
                    return 1;
                }
                specs += 1;
            }
            "acceptance_criterion" => {
                let id = json_str(&value["id"]);
                let spec_id = json_str(&value["spec_id"]);
                let slug = json_str(&value["slug"]);
                let title = json_str(&value["title"]);
                let intent = json_str(&value["intent"]);
                let review_mode = json_str(&value["review_mode"]);
                let risk_level = json_str(&value["risk_level"]);
                let created_at = json_str(&value["created_at"]);
                let updated_at = json_str(&value["updated_at"]);

                let q = format!(
                    "INSERT INTO acceptance_criteria (id, spec_id, slug, title, intent, review_mode, risk_level, created_at, updated_at) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}')",
                    escape(&id), escape(&spec_id), escape(&slug), escape(&title), escape(&intent), escape(&review_mode), escape(&risk_level), escape(&created_at), escape(&updated_at)
                );
                if let Err(e) = conn.query_drop(&q) {
                    eprintln!("import-jsonl: INSERT ac {id} failed: {e}");
                    return 1;
                }
                acs += 1;
            }
            "spec_relation" => {
                let id = json_str(&value["id"]);
                let source_spec_id = json_str(&value["source_spec_id"]);
                let target_spec_id = json_str(&value["target_spec_id"]);
                let relation_kind = json_str(&value["relation_kind"]);
                let created_at = json_str(&value["created_at"]);
                let updated_at = json_str(&value["updated_at"]);

                let q = format!(
                    "INSERT INTO spec_relations (id, source_spec_id, target_spec_id, relation_kind, created_at, updated_at) VALUES ('{}', '{}', '{}', '{}', '{}', '{}')",
                    escape(&id), escape(&source_spec_id), escape(&target_spec_id), escape(&relation_kind), escape(&created_at), escape(&updated_at)
                );
                if let Err(e) = conn.query_drop(&q) {
                    eprintln!("import-jsonl: INSERT rel {id} failed: {e}");
                    return 1;
                }
                rels += 1;
            }
            "acceptance_criterion_concern" => {
                let ac_id = json_str(&value["ac_id"]);
                let concern_kind = json_str(&value["concern_kind"]);

                let q = format!(
                    "INSERT INTO acceptance_criterion_concerns (ac_id, concern_kind) VALUES ('{}', '{}')",
                    escape(&ac_id), escape(&concern_kind)
                );
                if let Err(e) = conn.query_drop(&q) {
                    eprintln!("import-jsonl: INSERT concern {ac_id}/{concern_kind} failed: {e}");
                    return 1;
                }
                concerns += 1;
            }
            "codeintel_code_location" => {
                let id = json_str(&value["id"]);
                let repo_path = json_str(&value["repo_path"]);
                let file_path = json_str(&value["file_path"]);
                let kind = json_str(&value["kind"]);
                let symbol = json_str(&value["symbol"]);
                let test_command = json_str(&value["test_command"]);
                let created_at = json_str(&value["created_at"]);
                let updated_at = json_str(&value["updated_at"]);

                let q = format!(
                    "INSERT INTO codeintel_code_locations (id, repo_path, file_path, kind, symbol, test_command, created_at, updated_at) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}')",
                    escape(&id), escape(&repo_path), escape(&file_path), escape(&kind), escape(&symbol), escape(&test_command), escape(&created_at), escape(&updated_at)
                );
                if let Err(e) = conn.query_drop(&q) {
                    eprintln!("import-jsonl: INSERT location {id} failed: {e}");
                    return 1;
                }
                locations += 1;
            }
            "codeintel_ac_link" => {
                let id = json_str(&value["id"]);
                let ac_id = json_str(&value["ac_id"]);
                let code_location_id = json_str(&value["code_location_id"]);
                let relation_kind = json_str(&value["relation_kind"]);
                let note = json_str(&value["note"]);
                let created_at = json_str(&value["created_at"]);
                let updated_at = json_str(&value["updated_at"]);

                let q = format!(
                    "INSERT INTO codeintel_ac_links (id, ac_id, code_location_id, relation_kind, note, created_at, updated_at) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}')",
                    escape(&id), escape(&ac_id), escape(&code_location_id), escape(&relation_kind), escape(&note), escape(&created_at), escape(&updated_at)
                );
                if let Err(e) = conn.query_drop(&q) {
                    eprintln!("import-jsonl: INSERT link {id} failed: {e}");
                    return 1;
                }
                links += 1;
            }
            other => {
                eprintln!("import-jsonl: unknown record type: {other}");
                return 1;
            }
        }
    }

    eprintln!("import-jsonl: {specs} specs, {acs} ACs, {rels} rels, {concerns} concerns, {locations} locations, {links} links");
    0
}

fn json_str(v: &serde_json::Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "''")
}