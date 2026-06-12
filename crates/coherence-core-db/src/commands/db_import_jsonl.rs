//! Imports all semantic catalog tables from JSONL.
//! Usage: `coherence-core-db db import-jsonl --env prod --in <file> --confirm`
//!
//! Imports: `specs`, `acceptance_criteria`, `spec_relations`,
//!          `acceptance_criterion_concerns`, `codeintel_code_locations`, `codeintel_ac_links`

#![allow(clippy::too_many_lines)]

use crate::db::{
    connect_without_database, mysql_quote_identifier, user_scoped_dolt_from_manifest,
    ConnectionConfig,
};
use crate::project_manifest;
use mysql::prelude::Queryable;
use std::io::BufRead;

struct ImportCounts {
    specs: usize,
    acs: usize,
    rels: usize,
    concerns: usize,
    locations: usize,
    links: usize,
}

fn parse_import_args(args: &[String]) -> Result<(String, String), i32> {
    let mut iter = args.iter();
    let mut target_env: Option<String> = None;
    let mut in_path: Option<String> = None;
    let mut confirm = false;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--env" => {
                target_env = Some(
                    iter.next()
                        .map(String::as_str)
                        .unwrap_or_default()
                        .to_string(),
                );
            }
            "--in" => {
                in_path = Some(
                    iter.next()
                        .map(String::as_str)
                        .unwrap_or_default()
                        .to_string(),
                );
            }
            "--confirm" => confirm = true,
            other => {
                eprintln!("import-jsonl: unknown flag: {other}");
                return Err(1);
            }
        }
    }

    let target_env = match target_env {
        Some(e) if e == "dev" || e == "test" || e == "prod" => e,
        Some(e) => {
            eprintln!("import-jsonl: --env must be dev|test|prod (got {e})");
            return Err(1);
        }
        None => {
            eprintln!("import-jsonl: --env dev|test|prod is required");
            return Err(1);
        }
    };

    let in_path = in_path.ok_or_else(|| {
        eprintln!("import-jsonl: --in <file> is required");
        1
    })?;

    if !confirm {
        eprintln!("import-jsonl --env {target_env} --in {in_path}: --confirm is required");
        return Err(1);
    }

    Ok((target_env, in_path))
}

fn setup_import_connection(target_env: &str) -> Result<mysql::Conn, i32> {
    let manifest = project_manifest::try_read_project_manifest_from_cwd();
    if !user_scoped_dolt_from_manifest(&manifest) {
        eprintln!("import-jsonl: skipped (dolt_mode is not user-scoped)");
        return Err(0);
    }

    std::env::set_var("COHERENCE_ENV", target_env);
    let config = ConnectionConfig::from_env().map_err(|err| {
        eprintln!("import-jsonl: ConnectionConfig::from_env() failed: {err}");
        1
    })?;

    let (mut conn, _) = connect_without_database(&config).map_err(|e| {
        eprintln!("import-jsonl: connect failed: {e}");
        1
    })?;

    let ident = mysql_quote_identifier(&config.database);
    conn.query_drop(format!("USE {ident}")).map_err(|e| {
        eprintln!("import-jsonl: USE {ident} failed: {e}");
        1
    })?;

    Ok(conn)
}

pub fn run(args: &[String]) -> i32 {
    let (target_env, in_path) = match parse_import_args(args) {
        Ok(v) => v,
        Err(code) => return code,
    };

    let mut conn = match setup_import_connection(&target_env) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let file = match std::fs::File::open(&in_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("import-jsonl: cannot open {in_path}: {e}");
            return 1;
        }
    };

    let counts = match process_jsonl(&mut conn, std::io::BufReader::new(file)) {
        Ok(c) => c,
        Err(code) => return code,
    };

    eprintln!(
        "import-jsonl: {} specs, {} ACs, {} rels, {} concerns, {} locations, {} links",
        counts.specs, counts.acs, counts.rels, counts.concerns, counts.locations, counts.links
    );
    0
}

fn process_jsonl(
    conn: &mut mysql::Conn,
    reader: std::io::BufReader<std::fs::File>,
) -> Result<ImportCounts, i32> {
    let mut counts = ImportCounts {
        specs: 0,
        acs: 0,
        rels: 0,
        concerns: 0,
        locations: 0,
        links: 0,
    };

    for line in reader.lines() {
        let line = line.map_err(|e| {
            eprintln!("import-jsonl: read error: {e}");
            1
        })?;

        if line.trim().is_empty() {
            continue;
        }

        let value: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
            eprintln!("import-jsonl: parse error: {e}");
            1
        })?;

        handle_record(conn, &value, &mut counts)?;
    }

    Ok(counts)
}

fn handle_record(
    conn: &mut mysql::Conn,
    value: &serde_json::Value,
    counts: &mut ImportCounts,
) -> Result<(), i32> {
    let rec_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match rec_type {
        "spec" => import_spec(conn, value, counts),
        "acceptance_criterion" => import_ac(conn, value, counts),
        "spec_relation" => import_relation(conn, value, counts),
        "acceptance_criterion_concern" => import_concern(conn, value, counts),
        "codeintel_code_location" => import_code_location(conn, value, counts),
        "codeintel_ac_link" => import_ac_link(conn, value, counts),
        other => {
            eprintln!("import-jsonl: unknown record type: {other}");
            Err(1)
        }
    }
}

fn import_spec(
    conn: &mut mysql::Conn,
    value: &serde_json::Value,
    counts: &mut ImportCounts,
) -> Result<(), i32> {
    let id = json_str(&value["id"]);
    let q = format!(
        "INSERT INTO specs (id, slug, level, status, title, description, created_at, updated_at) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}')",
        escape(&id), escape(&json_str(&value["slug"])), escape(&json_str(&value["level"])), escape(&json_str(&value["status"])), escape(&json_str(&value["title"])), escape(&json_str(&value["description"])), escape(&json_str(&value["created_at"])), escape(&json_str(&value["updated_at"]))
    );
    exec_insert(conn, &q, "spec", &id)?;
    counts.specs += 1;
    Ok(())
}

fn import_ac(
    conn: &mut mysql::Conn,
    value: &serde_json::Value,
    counts: &mut ImportCounts,
) -> Result<(), i32> {
    let id = json_str(&value["id"]);
    let q = format!(
        "INSERT INTO acceptance_criteria (id, spec_id, slug, title, intent, review_mode, risk_level, created_at, updated_at) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}')",
        escape(&id), escape(&json_str(&value["spec_id"])), escape(&json_str(&value["slug"])), escape(&json_str(&value["title"])), escape(&json_str(&value["intent"])), escape(&normalize_review_mode(&json_str(&value["review_mode"]))), escape(&normalize_risk_level(&json_str(&value["risk_level"]))), escape(&json_str(&value["created_at"])), escape(&json_str(&value["updated_at"]))
    );
    exec_insert(conn, &q, "ac", &id)?;
    counts.acs += 1;
    Ok(())
}

fn import_relation(
    conn: &mut mysql::Conn,
    value: &serde_json::Value,
    counts: &mut ImportCounts,
) -> Result<(), i32> {
    let id = json_str(&value["id"]);
    let q = format!(
        "INSERT INTO spec_relations (id, source_spec_id, target_spec_id, relation_kind, created_at, updated_at) VALUES ('{}', '{}', '{}', '{}', '{}', '{}')",
        escape(&id), escape(&json_str(&value["source_spec_id"])), escape(&json_str(&value["target_spec_id"])), escape(&json_str(&value["relation_kind"])), escape(&json_str(&value["created_at"])), escape(&json_str(&value["updated_at"]))
    );
    exec_insert(conn, &q, "rel", &id)?;
    counts.rels += 1;
    Ok(())
}

fn import_concern(
    conn: &mut mysql::Conn,
    value: &serde_json::Value,
    counts: &mut ImportCounts,
) -> Result<(), i32> {
    let ac_id = json_str(&value["ac_id"]);
    let concern_kind = json_str(&value["concern_kind"]);
    let q = format!(
        "INSERT INTO acceptance_criterion_concerns (ac_id, concern_kind) VALUES ('{}', '{}')",
        escape(&ac_id),
        escape(&concern_kind)
    );
    conn.query_drop(&q).map_err(|e| {
        eprintln!("import-jsonl: INSERT concern {ac_id}/{concern_kind} failed: {e}");
        1
    })?;
    counts.concerns += 1;
    Ok(())
}

fn import_code_location(
    conn: &mut mysql::Conn,
    value: &serde_json::Value,
    counts: &mut ImportCounts,
) -> Result<(), i32> {
    let id = json_str(&value["id"]);
    let q = format!(
        "INSERT INTO codeintel_code_locations (id, repo_path, file_path, kind, symbol, test_command, created_at, updated_at) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}')",
        escape(&id), escape(&json_str(&value["repo_path"])), escape(&json_str(&value["file_path"])), escape(&json_str(&value["kind"])), escape(&json_str(&value["symbol"])), escape(&json_str(&value["test_command"])), escape(&json_str(&value["created_at"])), escape(&json_str(&value["updated_at"]))
    );
    exec_insert(conn, &q, "location", &id)?;
    counts.locations += 1;
    Ok(())
}

fn import_ac_link(
    conn: &mut mysql::Conn,
    value: &serde_json::Value,
    counts: &mut ImportCounts,
) -> Result<(), i32> {
    let id = json_str(&value["id"]);
    let q = format!(
        "INSERT INTO codeintel_ac_links (id, ac_id, code_location_id, relation_kind, note, created_at, updated_at) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}')",
        escape(&id), escape(&json_str(&value["ac_id"])), escape(&json_str(&value["code_location_id"])), escape(&json_str(&value["relation_kind"])), escape(&json_str(&value["note"])), escape(&json_str(&value["created_at"])), escape(&json_str(&value["updated_at"]))
    );
    exec_insert(conn, &q, "link", &id)?;
    counts.links += 1;
    Ok(())
}

fn exec_insert(conn: &mut mysql::Conn, q: &str, label: &str, id: &str) -> Result<(), i32> {
    conn.query_drop(q).map_err(|e| {
        eprintln!("import-jsonl: INSERT {label} {id} failed: {e}");
        1
    })
}

fn json_str(v: &serde_json::Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

fn normalize_review_mode(value: &str) -> String {
    match value {
        "" | "default" => "manual".to_string(),
        other => other.to_string(),
    }
}

fn normalize_risk_level(value: &str) -> String {
    match value {
        "" | "default" => "medium".to_string(),
        other => other.to_string(),
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_normalizes_legacy_default_ac_classification() {
        assert_eq!(normalize_review_mode("default"), "manual");
        assert_eq!(normalize_review_mode(""), "manual");
        assert_eq!(normalize_review_mode("hybrid"), "hybrid");
        assert_eq!(normalize_risk_level("default"), "medium");
        assert_eq!(normalize_risk_level(""), "medium");
        assert_eq!(normalize_risk_level("high"), "high");
    }
}
