//! Run shell commands linked to an acceptance criterion via `verified_by` codeintel links.
//! Loads links through [`crate::ac_code_link_store::list_code_links_for_ac`] (COREDB-11).

use std::path::PathBuf;
use std::process::{Command, Stdio};

use mysql::Conn;

use crate::ac_code_link_store;
use crate::models::{AcCodeLinkWithLocation, AcCodeRelationKind, CodeLocation, CodeLocationKind};

/// Maximum characters (Unicode scalar values) retained for per-link output summaries.
const OUTPUT_SUMMARY_MAX_CHARS: usize = 240;

/// Outcome for one verified-by link after evaluation (COREDB-13-friendly).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcVerifyLinkStatus {
    Passed,
    Failed { exit_code: i32 },
    Skipped { reason: String },
}

/// One evaluated link row: AC id, code location id, command string, status, short output text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcVerifyLinkRunRecord {
    pub ac_id: String,
    pub code_location_id: String,
    pub command: String,
    pub status: AcVerifyLinkStatus,
    pub output_summary: String,
}

/// Aggregate result for verifying a single AC (`verify-ac`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcVerifyAcRunResult {
    pub ac_id: String,
    /// True when this AC has no `verified_by` rows at all (distinct from “all skipped”).
    pub no_verification_links: bool,
    pub links: Vec<AcVerifyLinkRunRecord>,
}

impl AcVerifyAcRunResult {
    /// Exit code for the CLI: nonzero iff any executed command failed.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        i32::from(
            self.links
                .iter()
                .any(|r| matches!(r.status, AcVerifyLinkStatus::Failed { .. })),
        )
    }

    /// Overall label for stdout reporting (not spec-level).
    #[must_use]
    pub fn overall_status_label(&self) -> &'static str {
        if self.no_verification_links {
            return "no_verification";
        }
        if self
            .links
            .iter()
            .any(|r| matches!(r.status, AcVerifyLinkStatus::Failed { .. }))
        {
            "failed"
        } else {
            "passed"
        }
    }
}

/// Loads links via COREDB-11, filters `verified_by` + runnable kinds, runs `test_command` via `sh -c`.
///
/// # Errors
///
/// Returns [`Err`] when listing links fails or when spawning/waiting on a shell subprocess fails.
pub fn verify_acceptance_criterion(
    conn: &mut Conn,
    ac_id: &str,
) -> Result<AcVerifyAcRunResult, String> {
    let rows = ac_code_link_store::list_code_links_for_ac(conn, ac_id)?;
    let verified: Vec<&AcCodeLinkWithLocation> = rows
        .iter()
        .filter(|r| r.link.relation_kind == AcCodeRelationKind::VerifiedBy)
        .collect();

    let no_verification_links = verified.is_empty();

    let mut links = Vec::new();
    for row in verified {
        links.push(eval_verified_link(ac_id, row)?);
    }

    Ok(AcVerifyAcRunResult {
        ac_id: ac_id.to_string(),
        no_verification_links,
        links,
    })
}

fn eval_verified_link(
    ac_id: &str,
    row: &AcCodeLinkWithLocation,
) -> Result<AcVerifyLinkRunRecord, String> {
    let loc = &row.location;
    let code_location_id = loc.id.clone();

    if !matches!(
        loc.kind,
        CodeLocationKind::TestFile | CodeLocationKind::TestCommand
    ) {
        return Ok(AcVerifyLinkRunRecord {
            ac_id: ac_id.to_string(),
            code_location_id,
            command: String::new(),
            status: AcVerifyLinkStatus::Skipped {
                reason: format!(
                    "code location kind {:?} is not test_file or test_command",
                    loc.kind
                ),
            },
            output_summary: String::new(),
        });
    }

    let cmd_str = match loc.test_command.as_ref().map(|s| s.trim()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return Ok(AcVerifyLinkRunRecord {
                ac_id: ac_id.to_string(),
                code_location_id,
                command: String::new(),
                status: AcVerifyLinkStatus::Skipped {
                    reason: "missing test_command on code location".to_string(),
                },
                output_summary: String::new(),
            });
        }
    };

    let (code, summary) = run_shell_command(&cmd_str, loc)?;
    let status = if code == 0 {
        AcVerifyLinkStatus::Passed
    } else {
        AcVerifyLinkStatus::Failed { exit_code: code }
    };

    Ok(AcVerifyLinkRunRecord {
        ac_id: ac_id.to_string(),
        code_location_id,
        command: cmd_str,
        status,
        output_summary: summary,
    })
}

fn resolve_working_dir(loc: &CodeLocation) -> Option<PathBuf> {
    let repo_path = loc.repo_path.trim();
    if repo_path.is_empty() {
        return None;
    }
    let p = PathBuf::from(repo_path);
    if p.is_absolute() && p.exists() {
        return Some(p);
    }
    std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join(repo_path))
        .filter(|joined| joined.exists())
}

fn run_shell_command(cmd: &str, loc: &CodeLocation) -> Result<(i32, String), String> {
    let mut c = Command::new("sh");
    c.arg("-c").arg(cmd);
    c.stdin(Stdio::null());
    c.stdout(Stdio::piped());
    c.stderr(Stdio::piped());
    if let Some(dir) = resolve_working_dir(loc) {
        c.current_dir(dir);
    }

    let child = c.spawn().map_err(|e| format!("spawn failed: {e}"))?;
    let out = child
        .wait_with_output()
        .map_err(|e| format!("wait failed: {e}"))?;

    let code = out.status.code().unwrap_or(-1);
    let mut merged = String::new();
    merged.push_str(&String::from_utf8_lossy(&out.stdout));
    if !out.stderr.is_empty() {
        if !merged.is_empty() && !merged.ends_with('\n') {
            merged.push('\n');
        }
        merged.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    let summary = summarize_output(&merged);
    Ok((code, summary))
}

fn summarize_output(raw: &str) -> String {
    let flat: String = raw
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    let flat = flat.trim();
    let count = flat.chars().count();
    if count <= OUTPUT_SUMMARY_MAX_CHARS {
        flat.to_string()
    } else {
        let truncated: String = flat.chars().take(OUTPUT_SUMMARY_MAX_CHARS).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use mysql::Conn;

    use crate::ac_code_link_store;
    use crate::ac_verify::{verify_acceptance_criterion, AcVerifyLinkStatus};
    use crate::db::{self, ConnectionConfig};
    use crate::migrations;
    use crate::models::{
        AcCodeLink, AcCodeRelationKind, AcceptanceCriterion, CodeLocation, CodeLocationKind, Spec,
    };
    use crate::spec_store;

    fn maybe_conn() -> Option<Conn> {
        let config = ConnectionConfig::from_env();
        let _ = migrations::apply_all(&config).ok()?;
        db::connect(&config).ok().map(|(conn, _)| conn)
    }

    fn unique_label(prefix: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{prefix}-{nanos}-{}", std::process::id())
    }

    #[test]
    fn verify_ac_no_links_no_verification() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };
        let suf = unique_label("VFY-NONE");
        let ac_id = format!("AC-VFY-NONE-{suf}");

        let got = verify_acceptance_criterion(&mut conn, &ac_id).expect("verify");
        assert!(got.no_verification_links);
        assert!(got.links.is_empty());
        assert_eq!(got.exit_code(), 0);
        assert_eq!(got.overall_status_label(), "no_verification");
    }

    #[test]
    fn verify_ac_skips_non_verified_relation() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let suf = unique_label("VFY-SKIPREL");
        let ac_id = format!("AC-VFY-SR-{suf}");
        let loc_id = format!("LOC-VFY-SR-{suf}");
        let link_id = format!("LNK-VFY-SR-{suf}");

        let mut loc = CodeLocation::new(loc_id.clone(), ".", ".");
        loc.kind = CodeLocationKind::TestCommand;
        loc.test_command = Some("exit 1".to_string());
        loc.created_at = "t1".to_string();
        loc.updated_at = "t1".to_string();
        ac_code_link_store::put_code_location(&mut conn, &loc).expect("put_code_location");

        let mut link = AcCodeLink::new(
            link_id,
            ac_id.clone(),
            loc_id,
            AcCodeRelationKind::ImplementedBy,
        );
        link.created_at = "t2".to_string();
        link.updated_at = "t2".to_string();
        ac_code_link_store::put_ac_code_link(&mut conn, &link).expect("put_ac_code_link");

        let got = verify_acceptance_criterion(&mut conn, &ac_id).expect("verify");
        assert!(got.no_verification_links);
        assert!(got.links.is_empty());
        assert_eq!(got.exit_code(), 0);
    }

    #[test]
    fn verify_ac_skips_missing_command() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let suf = unique_label("VFY-NOCMD");
        let ac_id = format!("AC-VFY-NC-{suf}");
        let loc_id = format!("LOC-VFY-NC-{suf}");
        let link_id = format!("LNK-VFY-NC-{suf}");

        let mut loc = CodeLocation::new(loc_id.clone(), ".", ".");
        loc.kind = CodeLocationKind::TestFile;
        loc.test_command = None;
        loc.created_at = "t1".to_string();
        loc.updated_at = "t1".to_string();
        ac_code_link_store::put_code_location(&mut conn, &loc).expect("put_code_location");

        let mut link = AcCodeLink::new(
            link_id,
            ac_id.clone(),
            loc_id,
            AcCodeRelationKind::VerifiedBy,
        );
        link.created_at = "t2".to_string();
        link.updated_at = "t2".to_string();
        ac_code_link_store::put_ac_code_link(&mut conn, &link).expect("put_ac_code_link");

        let got = verify_acceptance_criterion(&mut conn, &ac_id).expect("verify");
        assert!(!got.no_verification_links);
        assert_eq!(got.links.len(), 1);
        assert!(matches!(
            got.links[0].status,
            AcVerifyLinkStatus::Skipped { .. }
        ));
        assert_eq!(got.exit_code(), 0);
    }

    #[test]
    fn verify_ac_passes_true_command() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let suf = unique_label("VFY-OK");
        let ac_id = format!("AC-VFY-OK-{suf}");
        let loc_id = format!("LOC-VFY-OK-{suf}");
        let link_id = format!("LNK-VFY-OK-{suf}");

        let mut loc = CodeLocation::new(loc_id.clone(), ".", ".");
        loc.kind = CodeLocationKind::TestCommand;
        loc.test_command = Some("true".to_string());
        loc.created_at = "t1".to_string();
        loc.updated_at = "t1".to_string();
        ac_code_link_store::put_code_location(&mut conn, &loc).expect("put_code_location");

        let mut link = AcCodeLink::new(
            link_id,
            ac_id.clone(),
            loc_id,
            AcCodeRelationKind::VerifiedBy,
        );
        link.created_at = "t2".to_string();
        link.updated_at = "t2".to_string();
        ac_code_link_store::put_ac_code_link(&mut conn, &link).expect("put_ac_code_link");

        let got = verify_acceptance_criterion(&mut conn, &ac_id).expect("verify");
        assert!(!got.no_verification_links);
        assert_eq!(got.links.len(), 1);
        assert_eq!(got.links[0].status, AcVerifyLinkStatus::Passed);
        assert_eq!(got.exit_code(), 0);
        assert_eq!(got.overall_status_label(), "passed");
    }

    #[test]
    fn verify_ac_fails_false_command() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let suf = unique_label("VFY-BAD");
        let ac_id = format!("AC-VFY-BAD-{suf}");
        let loc_id = format!("LOC-VFY-BAD-{suf}");
        let link_id = format!("LNK-VFY-BAD-{suf}");

        let mut loc = CodeLocation::new(loc_id.clone(), ".", ".");
        loc.kind = CodeLocationKind::TestCommand;
        loc.test_command = Some("false".to_string());
        loc.created_at = "t1".to_string();
        loc.updated_at = "t1".to_string();
        ac_code_link_store::put_code_location(&mut conn, &loc).expect("put_code_location");

        let mut link = AcCodeLink::new(
            link_id,
            ac_id.clone(),
            loc_id,
            AcCodeRelationKind::VerifiedBy,
        );
        link.created_at = "t2".to_string();
        link.updated_at = "t2".to_string();
        ac_code_link_store::put_ac_code_link(&mut conn, &link).expect("put_ac_code_link");

        let got = verify_acceptance_criterion(&mut conn, &ac_id).expect("verify");
        assert_eq!(got.links.len(), 1);
        assert!(matches!(
            got.links[0].status,
            AcVerifyLinkStatus::Failed { exit_code } if exit_code != 0
        ));
        assert_eq!(got.exit_code(), 1);
        assert_eq!(got.overall_status_label(), "failed");
    }

    #[test]
    fn verify_ac_round_trip_with_spec_row() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let suf = unique_label("VFY-SPEC");
        let spec_id = format!("SPEC-VFY-{suf}");
        let ac_id = format!("AC-VFY-SPEC-{suf}");
        let loc_id = format!("LOC-VFY-SPEC-{suf}");
        let link_id = format!("LNK-VFY-SPEC-{suf}");

        let mut spec = Spec::new(spec_id.clone(), "Verify AC runner");
        spec.description = "core-db verify-ac".to_string();
        spec.created_at = "ts".to_string();
        spec.updated_at = "ts".to_string();
        spec_store::put_spec(&mut conn, &spec).expect("put_spec");

        let mut ac = AcceptanceCriterion::new(ac_id.clone(), spec_id, "Runnable AC");
        ac.intent = "verified_by true".to_string();
        ac.created_at = "ta".to_string();
        ac.updated_at = "ta".to_string();
        spec_store::put_acceptance_criterion(&mut conn, &ac).expect("put_ac");

        let mut loc = CodeLocation::new(loc_id.clone(), ".", ".");
        loc.kind = CodeLocationKind::TestCommand;
        loc.test_command = Some("echo verify-ac-smoke".to_string());
        loc.created_at = "tl".to_string();
        loc.updated_at = "tl".to_string();
        ac_code_link_store::put_code_location(&mut conn, &loc).expect("put_code_location");

        let mut link = AcCodeLink::new(
            link_id,
            ac_id.clone(),
            loc_id,
            AcCodeRelationKind::VerifiedBy,
        );
        link.note = String::new();
        link.created_at = "tln".to_string();
        link.updated_at = "tln".to_string();
        ac_code_link_store::put_ac_code_link(&mut conn, &link).expect("put_ac_code_link");

        let got = verify_acceptance_criterion(&mut conn, &ac_id).expect("verify");
        assert_eq!(got.ac_id, ac_id);
        assert!(!got.no_verification_links);
        assert_eq!(got.links.len(), 1);
        assert_eq!(got.links[0].code_location_id, format!("LOC-VFY-SPEC-{suf}"));
        assert_eq!(got.links[0].command, "echo verify-ac-smoke");
        assert_eq!(got.links[0].status, AcVerifyLinkStatus::Passed);
        assert!(got.links[0].output_summary.contains("verify-ac-smoke"));
        assert_eq!(got.exit_code(), 0);
    }
}
