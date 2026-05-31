//! Latest verification-result cache for AC status rendering (`codeintel_*` tables).

use mysql::prelude::Queryable;
use mysql::{params, Conn};

use crate::ac_verify::{AcVerifyAcRunResult, AcVerifyLinkStatus, AcVerifyOverallStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcVerificationLatest {
    pub ac_id: String,
    pub overall_status: AcVerifyOverallStatus,
    pub no_verification_links: bool,
    pub link_count: usize,
    pub verified_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcVerificationLinkLatest {
    pub ac_id: String,
    pub code_location_id: String,
    pub command: String,
    pub status: AcVerifyLinkStatus,
    pub output_summary: String,
    pub verified_at: String,
}

pub fn put_ac_verification_result(
    conn: &mut Conn,
    result: &AcVerifyAcRunResult,
) -> Result<(), String> {
    let verified_at = now_label();
    conn.exec_drop(
        r"INSERT INTO codeintel_ac_verification_latest (
            ac_id,
            overall_status,
            no_verification_links,
            link_count,
            verified_at
          ) VALUES (
            :ac_id,
            :overall_status,
            :no_verification_links,
            :link_count,
            :verified_at
          )
          ON DUPLICATE KEY UPDATE
            overall_status = VALUES(overall_status),
            no_verification_links = VALUES(no_verification_links),
            link_count = VALUES(link_count),
            verified_at = VALUES(verified_at)",
        params! {
            "ac_id" => result.ac_id.as_str(),
            "overall_status" => result.overall_status_label(),
            "no_verification_links" => result.no_verification_links,
            "link_count" => result.links.len() as u64,
            "verified_at" => verified_at.as_str(),
        },
    )
    .map_err(|err| {
        format!(
            "failed to put verification latest for AC {}: {err}",
            result.ac_id
        )
    })?;

    conn.exec_drop(
        "DELETE FROM codeintel_ac_verification_link_latest WHERE ac_id = :ac_id",
        params! {
            "ac_id" => result.ac_id.as_str(),
        },
    )
    .map_err(|err| {
        format!(
            "failed to clear verification link latest for AC {}: {err}",
            result.ac_id
        )
    })?;

    for row in &result.links {
        let (status, exit_code, skip_reason) = link_status_columns(&row.status);
        conn.exec_drop(
            r"INSERT INTO codeintel_ac_verification_link_latest (
                ac_id,
                code_location_id,
                command,
                status,
                exit_code,
                skip_reason,
                output_summary,
                verified_at
              ) VALUES (
                :ac_id,
                :code_location_id,
                :command,
                :status,
                :exit_code,
                :skip_reason,
                :output_summary,
                :verified_at
              )",
            params! {
                "ac_id" => row.ac_id.as_str(),
                "code_location_id" => row.code_location_id.as_str(),
                "command" => row.command.as_str(),
                "status" => status,
                "exit_code" => exit_code,
                "skip_reason" => skip_reason,
                "output_summary" => row.output_summary.as_str(),
                "verified_at" => verified_at.as_str(),
            },
        )
        .map_err(|err| {
            format!(
                "failed to put verification link latest for AC {} location {}: {err}",
                row.ac_id, row.code_location_id
            )
        })?;
    }

    Ok(())
}

pub fn get_ac_verification_latest(
    conn: &mut Conn,
    ac_id: &str,
) -> Result<Option<AcVerificationLatest>, String> {
    if !table_exists(conn, "codeintel_ac_verification_latest")? {
        return Ok(None);
    }

    let row: Option<(String, String, bool, u64, String)> = conn
        .exec_first(
            r"SELECT ac_id, overall_status, no_verification_links, link_count, verified_at
              FROM codeintel_ac_verification_latest
              WHERE ac_id = :ac_id",
            params! {
                "ac_id" => ac_id,
            },
        )
        .map_err(|err| format!("failed to get verification latest for AC {ac_id}: {err}"))?;

    row.map(
        |(ac_id, overall_status, no_verification_links, link_count, verified_at)| {
            let overall_status = overall_status_from_label(&overall_status)?;
            let link_count = usize::try_from(link_count).map_err(|err| {
                format!("verification link_count out of range for AC {ac_id}: {err}")
            })?;
            Ok(AcVerificationLatest {
                ac_id,
                overall_status,
                no_verification_links,
                link_count,
                verified_at,
            })
        },
    )
    .transpose()
}

pub fn list_ac_verification_link_latest(
    conn: &mut Conn,
    ac_id: &str,
) -> Result<Vec<AcVerificationLinkLatest>, String> {
    type Row = (
        String,
        String,
        String,
        String,
        Option<i32>,
        Option<String>,
        String,
        String,
    );

    if !table_exists(conn, "codeintel_ac_verification_link_latest")? {
        return Ok(Vec::new());
    }

    let rows: Vec<Row> = conn
        .exec(
            r"SELECT ac_id, code_location_id, command, status, exit_code, skip_reason, output_summary, verified_at
              FROM codeintel_ac_verification_link_latest
              WHERE ac_id = :ac_id
              ORDER BY code_location_id",
            params! {
                "ac_id" => ac_id,
            },
        )
        .map_err(|err| format!("failed to list verification link latest for AC {ac_id}: {err}"))?;

    rows.into_iter()
        .map(
            |(
                ac_id,
                code_location_id,
                command,
                status,
                exit_code,
                skip_reason,
                output_summary,
                verified_at,
            )| {
                Ok(AcVerificationLinkLatest {
                    ac_id,
                    code_location_id,
                    command,
                    status: link_status_from_columns(&status, exit_code, skip_reason)?,
                    output_summary,
                    verified_at,
                })
            },
        )
        .collect()
}

fn table_exists(conn: &mut Conn, table_name: &str) -> Result<bool, String> {
    let count: Option<u64> = conn
        .exec_first(
            r"SELECT COUNT(*)
              FROM information_schema.tables
              WHERE table_schema = DATABASE()
                AND table_name = :table_name",
            params! {
                "table_name" => table_name,
            },
        )
        .map_err(|err| format!("failed to check table {table_name}: {err}"))?;
    Ok(count.unwrap_or(0) > 0)
}

fn link_status_columns(status: &AcVerifyLinkStatus) -> (&'static str, Option<i32>, Option<String>) {
    match status {
        AcVerifyLinkStatus::Passed => ("passed", None, None),
        AcVerifyLinkStatus::Failed { exit_code } => ("failed", Some(*exit_code), None),
        AcVerifyLinkStatus::Skipped { reason } => ("skipped", None, Some(reason.clone())),
    }
}

fn link_status_from_columns(
    status: &str,
    exit_code: Option<i32>,
    skip_reason: Option<String>,
) -> Result<AcVerifyLinkStatus, String> {
    match status {
        "passed" => Ok(AcVerifyLinkStatus::Passed),
        "failed" => Ok(AcVerifyLinkStatus::Failed {
            exit_code: exit_code.unwrap_or(-1),
        }),
        "skipped" => Ok(AcVerifyLinkStatus::Skipped {
            reason: skip_reason.unwrap_or_default(),
        }),
        _ => Err(format!("unknown verification link status: {status}")),
    }
}

fn overall_status_from_label(status: &str) -> Result<AcVerifyOverallStatus, String> {
    match status {
        "no_verification" => Ok(AcVerifyOverallStatus::NoVerification),
        "failed" => Ok(AcVerifyOverallStatus::Failed),
        "passed" => Ok(AcVerifyOverallStatus::Passed),
        "skipped" => Ok(AcVerifyOverallStatus::Skipped),
        _ => Err(format!("unknown verification overall status: {status}")),
    }
}

fn now_label() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    seconds.to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use mysql::Conn;

    use super::*;
    use crate::ac_verify::AcVerifyLinkRunRecord;
    use crate::db::{self, ConnectionConfig};
    use crate::migrations;
    use crate::test_world_guard;

    fn maybe_conn() -> Option<test_world_guard::EnvConnLock<Conn>> {
        let lock = test_world_guard::lock_test_env();
        let config = ConnectionConfig::from_env().ok()?;
        test_world_guard::panic_unless_isolated_test_world_for_writes(
            "ac_verification_store::tests",
            &config,
        );
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
            .map_or(0, |duration| duration.as_nanos());
        format!("{prefix}-{nanos}-{}", std::process::id())
    }

    #[test]
    fn latest_status_is_none_before_first_run() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };
        let ac_id = unique_label("AC-NOT-RUN");

        let latest = get_ac_verification_latest(&mut conn, &ac_id).expect("latest");

        assert!(latest.is_none());
    }

    #[test]
    fn put_and_get_latest_passed_result() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };
        let ac_id = unique_label("AC-PASS");
        let result = AcVerifyAcRunResult {
            ac_id: ac_id.clone(),
            no_verification_links: false,
            links: vec![AcVerifyLinkRunRecord {
                ac_id: ac_id.clone(),
                code_location_id: "loc-pass".into(),
                command: "true".into(),
                status: AcVerifyLinkStatus::Passed,
                output_summary: "ok".into(),
                captured_output: "ok".into(),
            }],
        };

        put_ac_verification_result(&mut conn, &result).expect("put");
        let latest = get_ac_verification_latest(&mut conn, &ac_id)
            .expect("latest")
            .expect("exists");
        let links = list_ac_verification_link_latest(&mut conn, &ac_id).expect("links");

        assert_eq!(latest.overall_status, AcVerifyOverallStatus::Passed);
        assert!(!latest.no_verification_links);
        assert_eq!(latest.link_count, 1);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].status, AcVerifyLinkStatus::Passed);
    }

    #[test]
    fn latest_result_replaces_old_link_rows() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };
        let ac_id = unique_label("AC-REPLACE");
        let first = AcVerifyAcRunResult {
            ac_id: ac_id.clone(),
            no_verification_links: false,
            links: vec![AcVerifyLinkRunRecord {
                ac_id: ac_id.clone(),
                code_location_id: "loc-old".into(),
                command: "false".into(),
                status: AcVerifyLinkStatus::Failed { exit_code: 1 },
                output_summary: "bad".into(),
                captured_output: "bad".into(),
            }],
        };
        let second = AcVerifyAcRunResult {
            ac_id: ac_id.clone(),
            no_verification_links: true,
            links: Vec::new(),
        };

        put_ac_verification_result(&mut conn, &first).expect("put first");
        put_ac_verification_result(&mut conn, &second).expect("put second");
        let latest = get_ac_verification_latest(&mut conn, &ac_id)
            .expect("latest")
            .expect("exists");
        let links = list_ac_verification_link_latest(&mut conn, &ac_id).expect("links");

        assert_eq!(latest.overall_status, AcVerifyOverallStatus::NoVerification);
        assert!(latest.no_verification_links);
        assert_eq!(latest.link_count, 0);
        assert!(links.is_empty());
    }
}
