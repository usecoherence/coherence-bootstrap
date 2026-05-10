//! `project init`: freeze `dolt_db_name` and `frozen_git_toplevel` in `.coherence/project.toml`.
//!
//! Requires `git` on `PATH`. Hash formula: first 4 lowercase hex chars of SHA-256 of UTF-8 bytes
//! of the trimmed `git rev-parse --show-toplevel` path; `dolt_db_name` = `sanitize(slug)_` + hash.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::project_manifest::{
    self, dolt_db_name_for_bind, read_manifest, write_manifest, ProjectManifest,
};

#[derive(Debug, Default)]
struct InitArgs {
    workspace: Option<PathBuf>,
    slug: Option<String>,
    force_rebind: bool,
}

pub fn run(args: &[String]) -> i32 {
    match run_impl(args) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("project init: {err}");
            1
        }
    }
}

fn run_impl(args: &[String]) -> Result<(), String> {
    let parsed = parse_init_argv(args)?;
    let workspace = parsed
        .workspace
        .clone()
        .map(Ok)
        .unwrap_or_else(|| env::current_dir().map_err(|e| format!("cwd: {e}")))?;

    let repo_root = project_manifest::find_git_repo_root(workspace.clone()).ok_or_else(|| {
        format!(
            "not inside a git repository (no .git under {}); try --workspace",
            workspace.display()
        )
    })?;

    let manifest_path = repo_root.join(".coherence").join("project.toml");
    let mut manifest = if manifest_path.is_file() {
        read_manifest(&repo_root)?
    } else {
        let slug = parsed.slug.clone().ok_or_else(|| {
            "missing .coherence/project.toml: pass --slug to create one".to_string()
        })?;
        ProjectManifest {
            version: 1,
            project_slug: slug,
            dolt_db_name: None,
            frozen_git_toplevel: None,
            project_hash: None,
        }
    };

    if manifest.project_slug.trim().is_empty() {
        return Err(
            "project_slug in manifest is empty; fix project.toml or use --slug with no manifest"
                .into(),
        );
    }

    if manifest.dolt_db_name.is_some() && !parsed.force_rebind {
        return Err(
            "dolt_db_name is already set in .coherence/project.toml; refuse to overwrite (use --force-rebind to recompute — old Dolt database on the server may be orphaned)"
                .into(),
        );
    }

    let toplevel = git_rev_parse_show_toplevel(&repo_root)?;
    let dolt_db = dolt_db_name_for_bind(&manifest.project_slug, &toplevel)?;

    manifest.dolt_db_name = Some(dolt_db);
    manifest.frozen_git_toplevel = Some(toplevel.clone());

    write_manifest(&repo_root, &manifest)?;
    println!("project_slug: {}", manifest.project_slug);
    println!(
        "dolt_db_name: {}",
        manifest.dolt_db_name.as_deref().unwrap_or("")
    );
    println!("frozen_git_toplevel: {toplevel}");
    Ok(())
}

/// Parse `project init` argv: `--workspace PATH`, `--slug SLUG`, `--force-rebind` (boolean).
fn parse_init_argv(args: &[String]) -> Result<InitArgs, String> {
    let mut out = InitArgs::default();
    let mut i = 0usize;
    while i < args.len() {
        let tok = &args[i];
        if tok == "--force-rebind" {
            out.force_rebind = true;
            i += 1;
        } else if tok == "--workspace" {
            let v = args
                .get(i + 1)
                .ok_or_else(|| "--workspace requires a path".to_string())?;
            if v.starts_with('-') && v != "-" {
                return Err("--workspace requires a path (next token looks like a flag)".into());
            }
            out.workspace = Some(PathBuf::from(v));
            i += 2;
        } else if tok == "--slug" {
            let v = args
                .get(i + 1)
                .ok_or_else(|| "--slug requires a value".to_string())?;
            if v.starts_with('-') {
                return Err("--slug requires a value (next token looks like a flag)".into());
            }
            out.slug = Some(v.clone());
            i += 2;
        } else {
            return Err(format!(
                "unexpected argument {:?} (try --workspace, --slug, --force-rebind)",
                tok
            ));
        }
    }
    Ok(out)
}

fn git_rev_parse_show_toplevel(repo_root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("failed to spawn `git`: {e} (is git on PATH?)"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "`git rev-parse --show-toplevel` failed in {} (is this a usable git work tree?): {}",
            repo_root.display(),
            stderr.trim()
        ));
    }
    let stdout =
        String::from_utf8(output.stdout).map_err(|e| format!("git stdout was not UTF-8: {e}"))?;
    let s = stdout.trim();
    if s.is_empty() {
        return Err("git rev-parse --show-toplevel returned empty stdout".into());
    }
    Ok(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_init_argv_force_and_workspace() {
        let a = parse_init_argv(&["--workspace".into(), "/tmp".into(), "--force-rebind".into()])
            .unwrap();
        assert!(a.force_rebind);
        assert_eq!(a.workspace, Some(PathBuf::from("/tmp")));
    }
}
