//! Repo-anchored Coherence project manifest (`.coherence/project.toml`).
//!
//! Discover the git repository root by walking parents until a `.git` file or directory exists
//! (no `git` subprocess). Read/write the manifest as TOML with serde.
//!
//! ## `dolt_db_name` sanitization
//!
//! [`sanitize_dolt_db_segment`] produces a **MySQL-style identifier segment**: lowercase ASCII
//! letters, digits, and underscores only, with runs of invalid characters collapsed to a single
//! underscore; leading/trailing underscores are trimmed. Output is capped at 64 bytes (common
//! MySQL identifier limit). Callers should treat an empty result as “nothing usable” and pick a
//! fallback.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const MANIFEST_REL_PATH: &str = ".coherence/project.toml";
const PROJECT_FILENAME: &str = "project.toml";

/// Maximum length for a sanitized Dolt/MySQL database name segment.
pub const DOLT_DB_NAME_MAX_LEN: usize = 64;

/// On-disk project manifest: identity and optional catalog binding (see design ADR / issue COREDB-2ft).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub version: u32,
    pub project_slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dolt_db_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frozen_git_toplevel: Option<String>,
}

/// Walk `start` and ancestors until a `.git` file or directory is found; return that directory.
pub fn find_git_repo_root(start: PathBuf) -> Option<PathBuf> {
    let mut dir = start;
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        dir = dir.parent()?.to_path_buf();
    }
}

/// Read `.coherence/project.toml` under `repo_root`.
pub fn read_manifest(repo_root: impl AsRef<Path>) -> Result<ProjectManifest, String> {
    let path = manifest_path(repo_root);
    let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let manifest: ProjectManifest =
        toml::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

/// Write `manifest` to `.coherence/project.toml` under `repo_root` (creates `.coherence/` as needed).
pub fn write_manifest(
    repo_root: impl AsRef<Path>,
    manifest: &ProjectManifest,
) -> Result<(), String> {
    validate_manifest(manifest)?;
    let coherence_dir = repo_root.as_ref().join(".coherence");
    fs::create_dir_all(&coherence_dir)
        .map_err(|e| format!("create {}: {e}", coherence_dir.display()))?;
    let path = coherence_dir.join(PROJECT_FILENAME);
    let serialized =
        toml::to_string_pretty(manifest).map_err(|e| format!("serialize manifest: {e}"))?;
    fs::write(&path, serialized).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}

fn manifest_path(repo_root: impl AsRef<Path>) -> PathBuf {
    repo_root.as_ref().join(MANIFEST_REL_PATH)
}

fn validate_manifest(manifest: &ProjectManifest) -> Result<(), String> {
    if manifest.project_slug.trim().is_empty() {
        return Err("project_slug must be non-empty".to_string());
    }
    Ok(())
}

/// Map arbitrary input to a lowercase ASCII `a-z0-9_` segment, max [`DOLT_DB_NAME_MAX_LEN`].
pub fn sanitize_dolt_db_segment(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        let mapped = match ch {
            'A'..='Z' => Some(ch.to_ascii_lowercase()),
            'a'..='z' | '0'..='9' => Some(ch),
            '_' => Some('_'),
            _ => None,
        };
        if let Some(c) = mapped {
            if c == '_' {
                if out.is_empty() {
                    continue;
                }
                if out.ends_with('_') {
                    continue;
                }
            }
            if out.len() >= DOLT_DB_NAME_MAX_LEN {
                break;
            }
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('_') && out.len() < DOLT_DB_NAME_MAX_LEN {
            out.push('_');
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.len() > DOLT_DB_NAME_MAX_LEN {
        out.truncate(DOLT_DB_NAME_MAX_LEN);
        while out.ends_with('_') {
            out.pop();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    use tempfile::TempDir;

    #[test]
    fn round_trip_write_read() {
        let tmp = TempDir::new().unwrap();
        let manifest = ProjectManifest {
            version: 1,
            project_slug: "my-project".to_string(),
            dolt_db_name: Some("my_catalog".to_string()),
            frozen_git_toplevel: Some("/tmp/repo".to_string()),
        };
        write_manifest(tmp.path(), &manifest).unwrap();
        let loaded = read_manifest(tmp.path()).unwrap();
        assert_eq!(loaded, manifest);
    }

    #[test]
    fn read_rejects_empty_slug() {
        let tmp = TempDir::new().unwrap();
        let bad = ProjectManifest {
            version: 1,
            project_slug: "   ".to_string(),
            dolt_db_name: None,
            frozen_git_toplevel: None,
        };
        assert!(write_manifest(tmp.path(), &bad).is_err());
    }

    #[test]
    fn find_git_repo_root_nested_with_git_dir() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join(".git")).unwrap();
        let nested = root.join("a/b/c");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(find_git_repo_root(nested), Some(root.to_path_buf()));
    }

    #[test]
    fn find_git_repo_root_nested_with_git_file() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let mut f = File::create(root.join(".git")).unwrap();
        writeln!(f, "gitdir: ../.git/modules/foo").unwrap();
        let nested = root.join("deep");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(find_git_repo_root(nested), Some(root.to_path_buf()));
    }

    #[test]
    fn find_git_repo_root_none_outside_repo() {
        let tmp = TempDir::new().unwrap();
        let lone = tmp.path().join("no_git");
        fs::create_dir_all(&lone).unwrap();
        assert!(find_git_repo_root(lone).is_none());
    }

    #[test]
    fn sanitize_dolt_db_segment_cases() {
        assert_eq!(sanitize_dolt_db_segment("MyCatalog_01"), "mycatalog_01");
        assert_eq!(sanitize_dolt_db_segment("a--b__c"), "a_b_c");
        assert_eq!(sanitize_dolt_db_segment("___leading"), "leading");
        assert_eq!(sanitize_dolt_db_segment("trailing___"), "trailing");
        assert_eq!(sanitize_dolt_db_segment("@@@"), "");
        assert_eq!(sanitize_dolt_db_segment(""), "");
        let long = "a".repeat(DOLT_DB_NAME_MAX_LEN + 20);
        let s = sanitize_dolt_db_segment(&long);
        assert_eq!(s.len(), DOLT_DB_NAME_MAX_LEN);
        assert!(s.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_')));
    }
}
