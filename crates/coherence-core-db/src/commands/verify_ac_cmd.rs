use crate::ac_verify::{verify_acceptance_criterion, AcVerifyLinkStatus};
use crate::commands::cli_parse::parse_args;
use crate::db::{connect, ConnectionConfig};
use crate::migrations;

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
    let config = ConnectionConfig::from_env();
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

fn escape_tab_field(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\t' | '\n' => ' ',
            o => o,
        })
        .collect()
}
