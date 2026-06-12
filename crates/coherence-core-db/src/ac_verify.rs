//! Run shell commands linked to an acceptance criterion via `verified_by` codeintel links.
//! Loads links through [`crate::ac_code_link_store::list_code_links_for_ac`] (COREDB-11).
//! Implemented CLI surfaces: `verify-ac`, `verify-spec` (`AGENTS.md` § M1 module ownership).

use std::path::PathBuf;
use std::process::{Command, Stdio};

use mysql::Conn;

use crate::ac_code_link_store;
use crate::ac_verification_store;
use crate::models::{AcCodeLinkWithLocation, AcCodeRelationKind, CodeLocation, CodeLocationKind};
use crate::spec_store;

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
    /// Merged stdout/stderr (capped) for ADR-0005 artifact capture; empty when the shell was not run.
    pub captured_output: String,
}

/// Per-AC aggregate status (`verify-ac` / `verify-spec`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcVerifyOverallStatus {
    NoVerification,
    Failed,
    Passed,
    Skipped,
}

impl AcVerifyOverallStatus {
    #[must_use]
    pub const fn as_label(&self) -> &'static str {
        match self {
            Self::NoVerification => "no_verification",
            Self::Failed => "failed",
            Self::Passed => "passed",
            Self::Skipped => "skipped",
        }
    }
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

    /// Per-AC rollup for reporting and spec-level counts.
    #[must_use]
    pub fn overall_status(&self) -> AcVerifyOverallStatus {
        let has_failed = self
            .links
            .iter()
            .any(|r| matches!(r.status, AcVerifyLinkStatus::Failed { .. }));
        let has_passed = self
            .links
            .iter()
            .any(|r| matches!(r.status, AcVerifyLinkStatus::Passed));

        match (self.no_verification_links, has_failed, has_passed) {
            (true, _, _) => AcVerifyOverallStatus::NoVerification,
            (_, true, _) => AcVerifyOverallStatus::Failed,
            (_, _, true) => AcVerifyOverallStatus::Passed,
            _ => AcVerifyOverallStatus::Skipped,
        }
    }

    #[must_use]
    pub fn overall_status_label(&self) -> &'static str {
        self.overall_status().as_label()
    }
}

/// Aggregate over one spec’s acceptance criteria (`verify-spec`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifySpecRunResult {
    pub spec_id: String,
    pub acceptance_criteria: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub no_verification: usize,
    pub ac_results: Vec<AcVerifyAcRunResult>,
}

impl VerifySpecRunResult {
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        self.ac_results
            .iter()
            .map(AcVerifyAcRunResult::exit_code)
            .max()
            .unwrap_or(0)
    }
}

/// Loads the spec row, lists ACs in id order, runs [`verify_acceptance_criterion`] for each (COREDB-12 logic).
///
/// # Errors
///
/// Returns [`Err`] when the spec id is unknown, listing fails, or verification fails internally.
pub fn verify_spec(conn: &mut Conn, spec_id: &str) -> Result<VerifySpecRunResult, String> {
    if spec_store::get_spec(conn, spec_id)?.is_none() {
        return Err(format!("spec not found: {spec_id}"));
    }

    let ac_rows = spec_store::list_acceptance_criteria_for_spec(conn, spec_id)?;
    let mut ac_results = Vec::with_capacity(ac_rows.len());
    for ac in ac_rows {
        ac_results.push(verify_acceptance_criterion(conn, &ac.id)?);
    }

    let (passed, failed, skipped, no_verification) = count_overall_statuses(&ac_results);

    Ok(VerifySpecRunResult {
        spec_id: spec_id.to_string(),
        acceptance_criteria: ac_results.len(),
        passed,
        failed,
        skipped,
        no_verification,
        ac_results,
    })
}

fn count_overall_statuses(ac_results: &[AcVerifyAcRunResult]) -> (usize, usize, usize, usize) {
    ac_results.iter().fold(
        (0usize, 0usize, 0usize, 0usize),
        |(passed, failed, skipped, no_verification), r| match r.overall_status() {
            AcVerifyOverallStatus::Passed => (passed + 1, failed, skipped, no_verification),
            AcVerifyOverallStatus::Failed => (passed, failed + 1, skipped, no_verification),
            AcVerifyOverallStatus::Skipped => (passed, failed, skipped + 1, no_verification),
            AcVerifyOverallStatus::NoVerification => (passed, failed, skipped, no_verification + 1),
        },
    )
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
    let mut links = Vec::new();
    let mut no_verification_links = true;
    for row in rows
        .iter()
        .filter(|r| r.link.relation_kind == AcCodeRelationKind::VerifiedBy)
    {
        no_verification_links = false;
        links.push(eval_verified_link(ac_id, row)?);
    }

    let result = AcVerifyAcRunResult {
        ac_id: ac_id.to_string(),
        no_verification_links,
        links,
    };
    ac_verification_store::put_ac_verification_result(conn, &result)?;
    Ok(result)
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
            captured_output: String::new(),
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
                captured_output: String::new(),
            });
        }
    };

    let (code, summary, captured) = run_shell_command(&cmd_str, loc)?;
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
        captured_output: captured,
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

fn run_shell_command(cmd: &str, loc: &CodeLocation) -> Result<(i32, String, String), String> {
    const CAP: usize = 512 * 1024;
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
    let mut captured = merged;
    if captured.len() > CAP {
        captured.truncate(CAP);
        captured.push_str("\n… [truncated]");
    }
    let summary = summarize_output(&captured);
    Ok((code, summary, captured))
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
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use mysql::Conn;

    use crate::ac_code_link_store;
    use crate::ac_verification_store;
    use crate::ac_verify::{verify_acceptance_criterion, verify_spec, AcVerifyLinkStatus};
    use crate::db::{self, ConnectionConfig};
    use crate::migrations;
    use crate::models::{
        AcCodeLink, AcCodeRelationKind, AcceptanceCriterion, CodeLocation, CodeLocationKind, Spec,
    };
    use crate::spec_store;
    use crate::test_world_guard;

    fn maybe_conn() -> Option<test_world_guard::EnvConnLock<Conn>> {
        let lock = test_world_guard::lock_test_env();
        let config = ConnectionConfig::from_env().ok()?;
        test_world_guard::panic_unless_isolated_test_world_for_writes("ac_verify::tests", &config);
        migrations::apply_all(&config).ok()?;
        let (conn, _) = db::connect(&config).ok()?;
        Some(test_world_guard::EnvConnLock {
            _lock: lock,
            inner: conn,
        })
    }

    fn unique_label(prefix: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        format!("{prefix}-{nanos}-{}", std::process::id())
    }

    fn setup_verified_loc(
        conn: &mut Conn,
        suf: &str,
        prefix: &str,
        command: Option<&str>,
    ) -> (String, String) {
        let ac_id = format!("AC-{prefix}-{suf}");
        let loc_id = format!("LOC-{prefix}-{suf}");
        let link_id = format!("LNK-{prefix}-{suf}");
        let mut loc = CodeLocation::new(loc_id.clone(), ".", ".");
        loc.kind = CodeLocationKind::TestCommand;
        loc.test_command = command.map(String::from);
        loc.created_at = "t1".to_string();
        loc.updated_at = "t1".to_string();
        ac_code_link_store::put_code_location(conn, &loc).expect("put_code_location");
        let mut link = AcCodeLink::new(
            link_id,
            ac_id.clone(),
            loc_id,
            AcCodeRelationKind::VerifiedBy,
        );
        link.created_at = "t2".to_string();
        link.updated_at = "t2".to_string();
        ac_code_link_store::put_ac_code_link(conn, &link).expect("put_ac_code_link");
        (ac_id, suf.to_string())
    }

    fn setup_spec_and_ac(conn: &mut Conn, suf: &str, prefix: &str, title: &str) -> String {
        let spec_id = format!("SPEC-{prefix}-{suf}");
        let mut spec = Spec::new(spec_id.clone(), title);
        spec.description = "core-db verify-spec".to_string();
        spec.created_at = "ts".to_string();
        spec.updated_at = "ts".to_string();
        spec_store::put_spec(conn, &spec).expect("put_spec");
        spec_id
    }

    fn setup_ac_for_spec(conn: &mut Conn, ac_id: &str, spec_id: &str, title: &str) {
        let mut ac = AcceptanceCriterion::new(ac_id.to_string(), spec_id.to_string(), title);
        ac.intent = "verify-spec counts".to_string();
        ac.created_at = "ta".to_string();
        ac.updated_at = "ta".to_string();
        spec_store::put_acceptance_criterion(conn, &ac).expect("put_ac");
    }

    fn conn_or_skip() -> Option<test_world_guard::EnvConnLock<Conn>> {
        maybe_conn()
    }

    #[test]
    fn verify_ac_no_links_no_verification() {
        let Some(mut conn) = conn_or_skip() else {
            return;
        };
        let suf = unique_label("VFY-NONE");
        let ac_id = format!("AC-VFY-NONE-{suf}");
        let got = verify_acceptance_criterion(&mut conn, &ac_id).expect("verify");
        assert_eq!(
            (
                got.no_verification_links,
                got.links.len(),
                got.exit_code(),
                got.overall_status_label()
            ),
            (true, 0, 0, "no_verification")
        );
    }

    #[test]
    fn verify_ac_skips_non_verified_relation() {
        let Some(mut conn) = conn_or_skip() else {
            return;
        };
        let suf = unique_label("VFY-SKIPREL");
        let ac_id = format!("AC-VFY-SR-{suf}");
        let loc_id = format!("LOC-VFY-SR-{suf}");
        let mut loc = CodeLocation::new(loc_id.clone(), ".", ".");
        loc.kind = CodeLocationKind::TestCommand;
        loc.test_command = Some("exit 1".to_string());
        loc.created_at = "t1".to_string();
        loc.updated_at = "t1".to_string();
        ac_code_link_store::put_code_location(&mut conn, &loc).expect("put_code_location");
        let mut link = AcCodeLink::new(
            format!("LNK-VFY-SR-{suf}"),
            ac_id.clone(),
            loc_id,
            AcCodeRelationKind::ImplementedBy,
        );
        link.created_at = "t2".to_string();
        link.updated_at = "t2".to_string();
        ac_code_link_store::put_ac_code_link(&mut conn, &link).expect("put_ac_code_link");
        let got = verify_acceptance_criterion(&mut conn, &ac_id).expect("verify");
        assert_eq!(
            (got.no_verification_links, got.links.len(), got.exit_code()),
            (true, 0, 0)
        );
    }

    #[test]
    fn verify_ac_skips_missing_command() {
        let Some(mut conn) = conn_or_skip() else {
            return;
        };
        let suf = unique_label("VFY-NOCMD");
        let ac_id = format!("AC-VFY-NC-{suf}");
        let loc_id = format!("LOC-VFY-NC-{suf}");
        let mut loc = CodeLocation::new(loc_id.clone(), ".", ".");
        loc.kind = CodeLocationKind::TestFile;
        loc.test_command = None;
        loc.created_at = "t1".to_string();
        loc.updated_at = "t1".to_string();
        ac_code_link_store::put_code_location(&mut conn, &loc).expect("put_code_location");
        let mut link = AcCodeLink::new(
            format!("LNK-VFY-NC-{suf}"),
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
        assert_eq!(
            (got.exit_code(), got.overall_status_label()),
            (0, "skipped")
        );
    }

    #[test]
    fn verify_ac_passes_true_command() {
        let Some(mut conn) = conn_or_skip() else {
            return;
        };
        let suf = unique_label("VFY-OK");
        let (ac_id, _) = setup_verified_loc(&mut conn, &suf, "VFY-OK", Some("true"));
        let got = verify_acceptance_criterion(&mut conn, &ac_id).expect("verify");
        assert!(!got.no_verification_links);
        assert_eq!(got.links.len(), 1);
        assert_eq!(got.links[0].status, AcVerifyLinkStatus::Passed);
        assert_eq!(got.exit_code(), 0);
        assert_eq!(got.overall_status_label(), "passed");
        let latest = ac_verification_store::get_ac_verification_latest(&mut conn, &ac_id)
            .expect("latest")
            .expect("recorded");
        assert_eq!(latest.overall_status.as_label(), "passed");
    }

    #[test]
    fn verify_ac_fails_false_command() {
        let Some(mut conn) = conn_or_skip() else {
            return;
        };
        let suf = unique_label("VFY-BAD");
        let (ac_id, _) = setup_verified_loc(&mut conn, &suf, "VFY-BAD", Some("false"));
        let got = verify_acceptance_criterion(&mut conn, &ac_id).expect("verify");
        assert_eq!(got.links.len(), 1);
        assert!(
            matches!(got.links[0].status, AcVerifyLinkStatus::Failed { exit_code } if exit_code != 0)
        );
        assert_eq!((got.exit_code(), got.overall_status_label()), (1, "failed"));
    }

    #[test]
    fn verify_ac_round_trip_with_spec_row() {
        let Some(mut conn) = conn_or_skip() else {
            return;
        };
        let suf = unique_label("VFY-SPEC");
        let spec_id = setup_spec_and_ac(&mut conn, &suf, "VFY", "Verify AC runner");
        let ac_id = format!("AC-VFY-SPEC-{suf}");
        setup_ac_for_spec(&mut conn, &ac_id, &spec_id, "Runnable AC");
        let loc_id = format!("LOC-VFY-SPEC-{suf}");
        let mut loc = CodeLocation::new(loc_id.clone(), ".", ".");
        loc.kind = CodeLocationKind::TestCommand;
        loc.test_command = Some("echo verify-ac-smoke".to_string());
        loc.created_at = "tl".to_string();
        loc.updated_at = "tl".to_string();
        ac_code_link_store::put_code_location(&mut conn, &loc).expect("put_code_location");
        let mut link = AcCodeLink::new(
            format!("LNK-VFY-SPEC-{suf}"),
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

    #[test]
    fn verify_spec_requires_spec_row() {
        let Some(mut conn) = conn_or_skip() else {
            return;
        };
        let suf = unique_label("VSPEC-NO");
        let spec_id = format!("SPEC-VS-NONE-{suf}");
        let err = verify_spec(&mut conn, &spec_id).expect_err("expected err");
        assert!(err.contains("not found"), "got {err}");
    }

    #[test]
    fn verify_spec_aggregate_counts_per_ac_status() {
        let Some(mut conn) = conn_or_skip() else {
            return;
        };
        let suf = unique_label("VSPEC-MIX");
        let spec_id = setup_spec_and_ac(&mut conn, &suf, "VS-MIX", "Verify-spec mix");
        let ac_no = format!("AC-VS-NV-{suf}");
        let ac_skip = format!("AC-VS-SK-{suf}");
        let ac_pass = format!("AC-VS-OK-{suf}");
        setup_ac_for_spec(&mut conn, &ac_no, &spec_id, "No links");
        setup_ac_for_spec(&mut conn, &ac_skip, &spec_id, "Skipped runner");
        setup_ac_for_spec(&mut conn, &ac_pass, &spec_id, "Passing runner");
        let _ = setup_verified_loc(&mut conn, &suf, "VS-SK", None);
        setup_verified_loc(&mut conn, &suf, "VS-OK", Some("true"));
        let report = verify_spec(&mut conn, &spec_id).expect("verify_spec");
        assert_eq!(
            (
                report.spec_id.as_str(),
                report.acceptance_criteria,
                report.no_verification,
                report.skipped,
                report.passed,
                report.failed,
                report.exit_code()
            ),
            (spec_id.as_str(), 3, 1, 1, 1, 0, 0)
        );
        assert_eq!(report.ac_results.len(), 3);
        assert!(report
            .ac_results
            .iter()
            .any(|r| r.ac_id == ac_no && r.overall_status_label() == "no_verification"));
        assert!(report
            .ac_results
            .iter()
            .any(|r| r.ac_id == ac_skip && r.overall_status_label() == "skipped"));
        assert!(report
            .ac_results
            .iter()
            .any(|r| r.ac_id == ac_pass && r.overall_status_label() == "passed"));
        let latest = ac_verification_store::get_ac_verification_latest(&mut conn, &ac_pass)
            .expect("latest")
            .expect("recorded");
        assert_eq!(latest.overall_status.as_label(), "passed");
    }

    #[test]
    fn verify_spec_nonzero_exit_when_any_command_fails() {
        let Some(mut conn) = conn_or_skip() else {
            return;
        };
        let suf = unique_label("VSPEC-FAIL");
        let spec_id = setup_spec_and_ac(&mut conn, &suf, "VS-FAIL", "Verify-spec fail");
        let ac_id = format!("AC-VS-FAIL-{suf}");
        setup_ac_for_spec(&mut conn, &ac_id, &spec_id, "Fails");
        setup_verified_loc(&mut conn, &suf, "VS-FAIL", Some("false"));
        let report = verify_spec(&mut conn, &spec_id).expect("verify_spec");
        assert_eq!((report.failed, report.exit_code()), (1, 1));
    }
}
