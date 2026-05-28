use crate::commands::cli_parse::{parse_args, ParsedArgs};
use coherence_core_db::db::{connect, ConnectionConfig};
use coherence_core_db::migrations;
use coherence_core_db::models::{slug_from_id, AcceptanceCriterion, ConcernKind, ReviewMode, RiskLevel};
use coherence_core_db::spec_store;

pub fn run(args: &[String]) -> i32 {
    match run_impl(args) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("ac: {err}");
            1
        }
    }
}

fn run_impl(args: &[String]) -> Result<(), String> {
    let sub = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| "usage: coherence-core-db ac <add|list|show> ...".to_string())?;
    let tail = &args[1..];
    match sub {
        "add" => ac_add(tail),
        "list" => ac_list(tail),
        "show" => ac_show(tail),
        other => Err(format!(
            "unknown ac subcommand: {other} (expected add, list, show)"
        )),
    }
}

fn utc_stamp() -> Result<String, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("clock error: {e}"))?
        .as_secs();
    Ok(format!("{secs}"))
}

fn gen_ac_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    format!("AC-GEN-{ms}")
}

fn connect_migrated() -> Result<mysql::Conn, String> {
    let config = ConnectionConfig::from_env()?;
    migrations::apply_all(&config)?;
    let (conn, _) = connect(&config)?;
    Ok(conn)
}

fn parse_concerns(p: &ParsedArgs) -> Result<Vec<ConcernKind>, String> {
    let mut out = Vec::new();
    for c in p.multi_flag("concern") {
        let kind = ConcernKind::from_db_str(c).ok_or_else(|| {
            format!(
                "unknown --concern {c:?} (correctness|security|performance|reliability|maintainability)"
            )
        })?;
        out.push(kind);
    }
    Ok(out)
}

fn ac_add(args: &[String]) -> Result<(), String> {
    let p = parse_args(args)?;
    let spec_id = p
        .single_flag("spec-id")?
        .ok_or_else(|| "--spec-id is required".to_string())?;
    let title = p
        .single_flag("title")?
        .ok_or_else(|| "--title is required".to_string())?;
    let intent = p.single_flag("intent")?.unwrap_or("");
    let review_str = p.single_flag("review-mode")?.unwrap_or("manual");
    let risk_str = p.single_flag("risk-level")?.unwrap_or("medium");
    let id = match p.single_flag("id")? {
        Some(id) => id.to_string(),
        None => gen_ac_id(),
    };
    let slug_flag = p.single_flag("slug")?;

    let review_mode = ReviewMode::from_db_str(review_str)
        .ok_or_else(|| format!("unknown --review-mode {review_str:?} (manual|automated|hybrid)"))?;
    let risk_level = RiskLevel::from_db_str(risk_str)
        .ok_or_else(|| format!("unknown --risk-level {risk_str:?} (low|medium|high|critical)"))?;
    let concerns = parse_concerns(&p)?;

    let mut conn = connect_migrated()?;
    if spec_store::get_spec(&mut conn, spec_id)?.is_none() {
        return Err(format!("spec not found: {spec_id}"));
    }

    let ts = utc_stamp()?;
    let mut ac = AcceptanceCriterion::new(id.clone(), spec_id, title);
    if let Some(s) = slug_flag {
        let slug = s.trim();
        if slug.is_empty() {
            return Err("--slug must not be empty when provided".to_string());
        }
        ac.slug = slug_from_id(slug);
    }
    ac.intent = intent.to_string();
    ac.review_mode = review_mode;
    ac.risk_level = risk_level;
    ac.concerns = concerns;
    ac.created_at.clone_from(&ts);
    ac.updated_at = ts;

    spec_store::put_acceptance_criterion(&mut conn, &ac)?;

    println!("ac_id: {}", ac.id);
    println!("spec_id: {}", ac.spec_id);
    println!("slug: {}", ac.slug);
    Ok(())
}

fn ac_list(args: &[String]) -> Result<(), String> {
    let p = parse_args(args)?;
    let spec_id = p
        .single_flag("spec-id")?
        .ok_or_else(|| "--spec-id is required".to_string())?;
    if !p.positionals.is_empty() {
        return Err(format!(
            "unexpected positional arguments to ac list: {:?}",
            p.positionals.join(" ")
        ));
    }

    let mut conn = connect_migrated()?;
    let acs = spec_store::list_acceptance_criteria_for_spec(&mut conn, spec_id)?;

    println!("id\tspec_id\tslug\ttitle\treview_mode\trisk_level\tconcerns");
    for ac in acs {
        let concerns: Vec<&str> = ac.concerns.iter().map(|k| k.as_db_str()).collect();
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            ac.id,
            ac.spec_id,
            ac.slug,
            ac.title.replace(['\t', '\n'], " "),
            ac.review_mode.as_db_str(),
            ac.risk_level.as_db_str(),
            concerns.join(","),
        );
    }
    Ok(())
}

fn ac_show(args: &[String]) -> Result<(), String> {
    let p = parse_args(args)?;
    if p.positionals.len() > 1 {
        return Err("ac show: expected at most one AC_ID positional".into());
    }
    let id_flag = p.single_flag("id")?;
    let id = match (id_flag, p.positionals.first()) {
        (Some(a), Some(b)) if a != b.as_str() => {
            return Err("ac show: --id and positional AC_ID disagree".into());
        }
        (Some(a), Some(_) | None) => a.to_string(),
        (None, Some(b)) => b.clone(),
        (None, None) => {
            return Err("ac show requires <AC_ID> or --id <AC_ID>".into());
        }
    };

    let mut conn = connect_migrated()?;
    let ac = spec_store::get_acceptance_criterion(&mut conn, &id)?
        .ok_or_else(|| format!("acceptance criterion not found: {id}"))?;

    println!("id: {}", ac.id);
    println!("spec_id: {}", ac.spec_id);
    println!("slug: {}", ac.slug);
    println!("title: {}", ac.title);
    println!("intent: {}", ac.intent);
    println!("review_mode: {}", ac.review_mode.as_db_str());
    println!("risk_level: {}", ac.risk_level.as_db_str());
    let concerns: Vec<&str> = ac.concerns.iter().map(|k| k.as_db_str()).collect();
    println!("concerns: {}", concerns.join(","));
    println!("created_at: {}", ac.created_at);
    println!("updated_at: {}", ac.updated_at);
    Ok(())
}
