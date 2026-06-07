use std::path::Path;

use crate::commands::cli_parse::parse_args;
use coherence_core_db::ac_verify::{
    verify_acceptance_criterion, AcVerifyAcRunResult, AcVerifyLinkStatus,
};
use coherence_core_db::db::{connect, ConnectionConfig};
use coherence_core_db::evidence_store::{self, RunLayout, SnapshotEnvelope, ARTIFACTS_SEGMENT};
use coherence_core_db::migrations;

pub fn run(args: &[String]) -> i32 {
    match run_impl(args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("verify-ac: {err}");
            1
        }
    }
}

fn connect_migrated() -> Result<mysql::Conn, String> {
    let config = ConnectionConfig::from_env()?;
    migrations::apply_all(&config)?;
    let (conn, _) = connect(&config)?;
    Ok(conn)
}

fn resolve_ac_id(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err(
            "usage: coherence-core-db verify-ac <AC_ID>\n       coherence-core-db verify-ac --ac-id <AC_ID>"
                .into(),
        );
    }
    if args[0].starts_with('-') {
        let p = parse_args(args)?;
        return p
            .single_flag("ac-id")?
            .map(str::to_string)
            .ok_or_else(|| "--ac-id is required when using flags".into());
    }
    if args.len() != 1 {
        return Err("usage: coherence-core-db verify-ac <AC_ID>".into());
    }
    Ok(args[0].clone())
}

fn run_impl(args: &[String]) -> Result<i32, String> {
    let ac_id = resolve_ac_id(args)?;
    let mut conn = connect_migrated()?;
    let result = verify_acceptance_criterion(&mut conn, &ac_id)?;

    if let Ok(run_id) = std::env::var("COHERENCE_EVIDENCE_RUN_ID") {
        let workspace = std::env::var("COHERENCE_WORKSPACE_ROOT")
            .ok()
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::current_dir().ok());
        if let Some(ws) = workspace {
            best_effort_verify_ac_evidence(&ws, &run_id, &result);
        }
    }

    println!(
        "OVERALL\tac_id={}\tstatus={}",
        result.ac_id,
        result.overall_status_label()
    );

    if result.no_verification_links {
        println!("SUMMARY\tno verified_by links for this AC");
    }

    for row in &result.links {
        let status_str = match &row.status {
            AcVerifyLinkStatus::Passed => "passed".to_string(),
            AcVerifyLinkStatus::Failed { exit_code } => format!("failed exit_code={exit_code}"),
            AcVerifyLinkStatus::Skipped { reason } => format!("skipped reason={reason}"),
        };
        println!(
            "LINK\tac_id={}\tcode_location_id={}\tcommand={}\tstatus={}\tsummary={}",
            row.ac_id,
            row.code_location_id,
            escape_tab_field(&row.command),
            escape_tab_field(&status_str),
            escape_tab_field(&row.output_summary),
        );
    }

    Ok(result.exit_code())
}

/// Writes one [`SnapshotEnvelope`] per executed link when `COHERENCE_EVIDENCE_RUN_ID` is set.
/// Failures are ignored so verification exit status is never blocked by evidence I/O.
fn best_effort_verify_ac_evidence(workspace: &Path, run_id: &str, result: &AcVerifyAcRunResult) {
    let layout = RunLayout::new(workspace.to_path_buf(), run_id.to_string());
    if evidence_store::ensure_run_initialized(&layout).is_err() {
        return;
    }

    let suffix = unique_obs_suffix();

    for (idx, row) in result.links.iter().enumerate() {
        if matches!(row.status, AcVerifyLinkStatus::Skipped { .. }) {
            continue;
        }

        let obs_id = format!(
            "obs-vfy-{}-{}-{}-{}",
            safe_segment(&row.code_location_id),
            safe_segment(&result.ac_id),
            suffix,
            idx
        );

        let status_label = match &row.status {
            AcVerifyLinkStatus::Passed => "passed",
            AcVerifyLinkStatus::Failed { .. } => "failed",
            AcVerifyLinkStatus::Skipped { .. } => unreachable!(),
        };
        let exit_code_json = match &row.status {
            AcVerifyLinkStatus::Passed => serde_json::json!(0),
            AcVerifyLinkStatus::Failed { exit_code } => serde_json::json!(*exit_code),
            AcVerifyLinkStatus::Skipped { .. } => serde_json::Value::Null,
        };

        let payload = if row.captured_output.is_empty() {
            serde_json::json!({
                "command": row.command,
                "status": status_label,
                "exit_code": exit_code_json,
                "output_summary": row.output_summary,
            })
        } else {
            serde_json::json!({
                "command": row.command,
                "status": status_label,
                "exit_code": exit_code_json,
                "output_summary": row.output_summary,
                "captured_byte_length": row.captured_output.len(),
            })
        };

        let (stdout_artifact_relpath, content_hash) = if row.captured_output.is_empty() {
            let Ok(h) = evidence_store::snapshot_payload_content_hash(&payload) else {
                continue;
            };
            (None, h)
        } else {
            let rel_under = format!("verify-ac/{obs_id}/stdout.txt");
            let Ok(hash) = evidence_store::write_bytes_under_artifacts(
                &layout,
                &rel_under,
                row.captured_output.as_bytes(),
            ) else {
                continue;
            };
            let full_rel = Path::new(ARTIFACTS_SEGMENT)
                .join(&rel_under)
                .to_string_lossy()
                .replace('\\', "/");
            (Some(full_rel), hash)
        };

        let envelope = SnapshotEnvelope {
            run_id: run_id.to_string(),
            observation_id: obs_id,
            object_kind: "process.output".to_string(),
            object_id: format!("verified_by:{}:{}", result.ac_id, row.code_location_id),
            payload,
            content_hash,
            ac_id: Some(result.ac_id.clone()),
            redaction_policy_id: None,
            stdout_artifact_relpath,
        };
        let _ = evidence_store::write_snapshot_envelope(&layout, &envelope);
    }
}

fn unique_obs_suffix() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
}

fn safe_segment(s: &str) -> String {
    if s.is_empty() {
        return "x".to_string();
    }
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn escape_tab_field(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\t' | '\n' => ' ',
            o => o,
        })
        .collect()
}
