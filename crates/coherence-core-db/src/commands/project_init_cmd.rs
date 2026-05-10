//! `project init`: bind `project_hash` once (4-hex from the frozen git worktree path), plus audit
//! `frozen_git_toplevel`, and persist legacy `dolt_db_name` derived from the same formula for
//! existing `ConnectionConfig` resolution.
//!
//! `git` on `PATH` is required only when the manifest has no usable `frozen_git_toplevel` yet
//! (first bind from a slug-only skeleton). Hash formula: first 4 lowercase hex chars of SHA-256 over
//! UTF-8 bytes of the trimmed path string (same as [`project_manifest::short_hash_frozen_git_path`]).

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::project_manifest::{
    self, dolt_db_name_for_bind, read_manifest, short_hash_frozen_git_path, write_manifest,
    ProjectManifest, CURRENT_MANIFEST_SCHEMA_VERSION,
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
        .map_or_else(|| env::current_dir().map_err(|e| format!("cwd: {e}")), Ok)?;

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

    if parsed.force_rebind {
        let toplevel = git_rev_parse_show_toplevel(&repo_root)?;
        apply_bind(&mut manifest, &toplevel)?;
        write_manifest(&repo_root, &manifest)?;
        print_summary(&manifest, &toplevel);
        return Ok(());
    }

    if manifest
        .project_hash
        .as_ref()
        .is_some_and(|h| !h.trim().is_empty())
    {
        let frozen = manifest
            .frozen_git_toplevel
            .clone()
            .unwrap_or_else(String::new);
        print_summary(&manifest, frozen.trim());
        return Ok(());
    }

    // project_hash missing: fill from frozen_git_toplevel if present, else run git once.
    let path_for_bind = if let Some(ref f) = manifest.frozen_git_toplevel {
        let t = f.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    } else {
        None
    };

    let toplevel = if let Some(p) = path_for_bind {
        p
    } else {
        if manifest.dolt_db_name.is_some() {
            return Err(
                "manifest has dolt_db_name but no project_hash and no frozen_git_toplevel; cannot derive hash (use --force-rebind after fixing the repo — old Dolt database on the server may be orphaned)"
                    .into(),
            );
        }
        git_rev_parse_show_toplevel(&repo_root)?
    };

    apply_bind(&mut manifest, &toplevel)?;
    write_manifest(&repo_root, &manifest)?;
    print_summary(&manifest, &toplevel);
    Ok(())
}

fn apply_bind(manifest: &mut ProjectManifest, frozen_git_path: &str) -> Result<(), String> {
    let hash = short_hash_frozen_git_path(frozen_git_path);
    let dolt_db = dolt_db_name_for_bind(&manifest.project_slug, frozen_git_path)?;
    manifest.version = manifest.version.max(CURRENT_MANIFEST_SCHEMA_VERSION);
    manifest.project_hash = Some(hash);
    manifest.frozen_git_toplevel = Some(frozen_git_path.trim().to_string());
    manifest.dolt_db_name = Some(dolt_db);
    Ok(())
}

fn print_summary(manifest: &ProjectManifest, frozen_toplevel_display: &str) {
    println!("project_slug: {}", manifest.project_slug);
    println!(
        "project_hash: {}",
        manifest.project_hash.as_deref().unwrap_or("")
    );
    println!(
        "dolt_db_name: {}",
        manifest.dolt_db_name.as_deref().unwrap_or("")
    );
    println!("frozen_git_toplevel: {frozen_toplevel_display}");
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
                "unexpected argument {tok:?} (try --workspace, --slug, --force-rebind)",
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
        let a = match parse_init_argv(&[
            "--workspace".into(),
            "/tmp".into(),
            "--force-rebind".into(),
        ]) {
            Ok(v) => v,
            Err(e) => panic!("parse_init_argv: {e}"),
        };
        assert!(a.force_rebind);
        assert_eq!(a.workspace, Some(PathBuf::from("/tmp")));
    }
}
