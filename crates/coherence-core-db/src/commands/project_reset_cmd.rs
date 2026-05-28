//! `project reset`: repair path toward a healthy first-migrate state without discarding
//! operator-authored **`project_slug`**. Runs idempotent **`project init`** (binds **`project_hash`**
//! when missing), then the same catalog preflight + database ensure + Refinery path as **`migrate`**.

use std::env;

use coherence_core_db::db::ConnectionConfig;
use coherence_core_db::migrations;
use coherence_core_db::project_manifest;

pub fn run() -> i32 {
    match run_impl() {
        Ok(()) => {
            println!("project reset: success");
            0
        }
        Err(err) => {
            eprintln!("project reset: {err}");
            1
        }
    }
}

fn run_impl() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let repo_root = project_manifest::find_git_repo_root(cwd).ok_or_else(|| {
        "not inside a git work tree — cd into your Coherence repository".to_string()
    })?;

    let manifest = project_manifest::read_manifest(&repo_root)?;
    if manifest.project_slug.trim().is_empty() {
        return Err(
            "project_slug is empty in `.coherence/project.toml`; set it before reset".into(),
        );
    }

    let init_code = crate::commands::project_init_cmd::run(&[]);
    if init_code != 0 {
        return Err("project init step failed (see stderr above)".into());
    }

    coherence_core_db::db::manifest_catalog_preflight_for_connect("project reset")?;

    let config = ConnectionConfig::from_env()?;
    let applied = migrations::apply_all(&config)?;
    println!("database: {}", config.database);
    println!("applied_migrations: {applied}");
    Ok(())
}
