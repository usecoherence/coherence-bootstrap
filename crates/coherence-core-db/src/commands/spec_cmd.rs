use crate::commands::cli_parse::parse_args;
use coherence_core_db::db::{connect, ConnectionConfig};
use coherence_core_db::migrations;
use coherence_core_db::models::{Spec, SpecLevel, SpecStatus};
use coherence_core_db::spec_store;

pub fn run(args: &[String]) -> i32 {
    match run_impl(args) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("spec: {err}");
            1
        }
    }
}

fn run_impl(args: &[String]) -> Result<(), String> {
    let sub = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| "usage: coherence-core-db spec <add|list|show> ...".to_string())?;
    let tail = &args[1..];
    match sub {
        "add" => spec_add(tail),
        "list" => spec_list(tail),
        "show" => spec_show(tail),
        other => Err(format!(
            "unknown spec subcommand: {other} (expected add, list, show)"
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

fn gen_spec_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    format!("SPEC-GEN-{ms}")
}

fn connect_migrated() -> Result<mysql::Conn, String> {
    let config = ConnectionConfig::from_env()?;
    migrations::apply_all(&config)?;
    let (conn, _) = connect(&config)?;
    Ok(conn)
}

fn spec_add(args: &[String]) -> Result<(), String> {
    let p = parse_args(args)?;
    let slug = p
        .single_flag("slug")?
        .ok_or_else(|| "--slug is required".to_string())?;
    let title = p
        .single_flag("title")?
        .ok_or_else(|| "--title is required".to_string())?;
    let description = p.single_flag("description")?.unwrap_or("");
    let level_str = p.single_flag("level")?.unwrap_or("module");
    let status_str = p.single_flag("status")?.unwrap_or("draft");
    let id = match p.single_flag("id")? {
        Some(id) => id.to_string(),
        None => gen_spec_id(),
    };

    let level = SpecLevel::from_db_str(level_str)
        .ok_or_else(|| format!("unknown --level {level_str:?} (product|system|module)"))?;
    let status = SpecStatus::from_db_str(status_str).ok_or_else(|| {
        format!("unknown --status {status_str:?} (draft|active|deprecated|archived)")
    })?;

    let ts = utc_stamp()?;
    let mut spec = Spec::new(id.clone(), title);
    spec.slug = slug.to_string();
    spec.description = description.to_string();
    spec.level = level;
    spec.status = status;
    spec.created_at.clone_from(&ts);
    spec.updated_at = ts;

    let mut conn = connect_migrated()?;
    spec_store::put_spec(&mut conn, &spec)?;

    println!("spec_id: {}", spec.id);
    println!("slug: {}", spec.slug);
    Ok(())
}

fn spec_list(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err(format!(
            "unexpected arguments to spec list: {:?}",
            args.join(" ")
        ));
    }
    let mut conn = connect_migrated()?;
    let specs = spec_store::list_specs(&mut conn)?;
    println!("id\tslug\ttitle\tlevel\tstatus");
    for s in specs {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            s.id,
            s.slug,
            s.title.replace(['\t', '\n'], " "),
            s.level.as_db_str(),
            s.status.as_db_str(),
        );
    }
    Ok(())
}

fn spec_show(args: &[String]) -> Result<(), String> {
    let p = parse_args(args)?;
    if p.positionals.len() > 1 {
        return Err("spec show: expected at most one SPEC_ID positional".into());
    }
    let id_flag = p.single_flag("id")?;
    let id = match (id_flag, p.positionals.first()) {
        (Some(a), Some(b)) if a != b.as_str() => {
            return Err("spec show: --id and positional SPEC_ID disagree".into());
        }
        (Some(a), Some(_) | None) => a.to_string(),
        (None, Some(b)) => b.clone(),
        (None, None) => {
            return Err("spec show requires <SPEC_ID> or --id <SPEC_ID>".into());
        }
    };

    let mut conn = connect_migrated()?;
    let spec =
        spec_store::get_spec(&mut conn, &id)?.ok_or_else(|| format!("spec not found: {id}"))?;

    println!("id: {}", spec.id);
    println!("slug: {}", spec.slug);
    println!("title: {}", spec.title);
    println!("description: {}", spec.description);
    println!("level: {}", spec.level.as_db_str());
    println!("status: {}", spec.status.as_db_str());
    println!("created_at: {}", spec.created_at);
    println!("updated_at: {}", spec.updated_at);
    Ok(())
}
