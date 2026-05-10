use crate::ac_verify::{verify_spec, AcVerifyLinkStatus};
use crate::commands::cli_parse::parse_args;
use crate::db::{connect, ConnectionConfig};
use crate::migrations;

pub fn run(args: &[String]) -> i32 {
    match run_impl(args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("verify-spec: {err}");
            1
        }
    }
}

fn connect_migrated() -> Result<mysql::Conn, String> {
    let config = ConnectionConfig::from_env();
    migrations::apply_all(&config)?;
    let (conn, _) = connect(&config)?;
    Ok(conn)
}

fn resolve_spec_id(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err(
            "usage: coherence-core-db verify-spec <SPEC_ID>\n       coherence-core-db verify-spec --spec-id <SPEC_ID>"
                .into(),
        );
    }
    if args[0].starts_with('-') {
        let p = parse_args(args)?;
        return p
            .single_flag("spec-id")?
            .map(str::to_string)
            .ok_or_else(|| "--spec-id is required when using flags".into());
    }
    if args.len() != 1 {
        return Err("usage: coherence-core-db verify-spec <SPEC_ID>".into());
    }
    Ok(args[0].clone())
}

fn run_impl(args: &[String]) -> Result<i32, String> {
    let spec_id = resolve_spec_id(args)?;
    let mut conn = connect_migrated()?;
    let report = verify_spec(&mut conn, &spec_id)?;

    println!(
        "SPEC\tspec_id={}\tspecs=1\tacceptance_criteria={}\tpassed={}\tfailed={}\tskipped={}\tno_verification={}",
        report.spec_id,
        report.acceptance_criteria,
        report.passed,
        report.failed,
        report.skipped,
        report.no_verification,
    );

    for result in &report.ac_results {
        println!(
            "AC\tspec_id={}\tac_id={}\tstatus={}",
            report.spec_id,
            result.ac_id,
            result.overall_status_label()
        );

        if result.no_verification_links {
            println!(
                "SUMMARY\tac_id={}\tno verified_by links for this AC",
                result.ac_id
            );
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
    }

    Ok(report.exit_code())
}

fn escape_tab_field(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\t' | '\n' => ' ',
            o => o,
        })
        .collect()
}
