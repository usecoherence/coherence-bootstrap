//! Repo-anchored Coherence project manifest (`.coherence/project.toml`).
//!
//! Discover the git repository root by walking parents until a `.git` file or directory exists
//! (no `git` subprocess). Read/write the manifest as TOML with serde.
//!
//! ## Manifest schema `version`
//!
//! - **Version 1**: `project_slug`, optional `dolt_db_name`, optional `frozen_git_toplevel`.
//! - **Version 2**: adds optional `project_hash`; when `project_hash` is present, `version` must
//!   be at least **2** ([`CURRENT_MANIFEST_SCHEMA_VERSION`]).
//!
//! ## `COHERENCE_ENV`
//!
//! Operators select a logical deployment tier with the **`COHERENCE_ENV`** environment variable.
//! Allowed values (ASCII, case-insensitive): **`dev`**, **`test`**, **`prod`**.
//!
//! When **`COHERENCE_ENV` is unset or empty**, interactive tooling treats the tier as **`dev`**
//! ([`parse_coherence_env`], [`coherence_env_from_std_env`]). Invalid non-empty values are
//! rejected with an error from [`coherence_env_from_std_env`].
//!
//! ## Effective Dolt catalog name (normalized)
//!
//! [`effective_dolt_catalog_name`] builds a single SQL-safe MySQL/Dolt **database identifier**; the
//! CLI wires it via [`crate::db::ConnectionConfig::from_env`] when **`DOLT_DB`** is unset and the
//! manifest has a bound **`project_hash`** (see `db.rs` / COREDB-6uf).
//!
//! 1. Sanitize [`sanitize_dolt_db_segment`] **slug** from `project_slug`.
//! 2. If `project_hash` is `Some` and non-whitespace, sanitize the trimmed hash as a middle **segment**
//!    (otherwise omit the segment).
//! 3. Append the **`env`** tier as a literal segment: `dev`, `test`, or `prod`.
//! 4. Join segments with a single underscore (`_`). If the joined string exceeds
//!    [`DOLT_DB_NAME_MAX_LEN`], the implementation shortens the slug and/or hash segments (never the
//!    env suffix) until the full name fits — same style as [`dolt_db_name_for_bind`].
//!
//! ## `dolt_db_name` sanitization
//!
//! [`sanitize_dolt_db_segment`] produces a **MySQL-style identifier segment**: lowercase ASCII
//! letters, digits, and underscores only, with runs of invalid characters collapsed to a single
//! underscore; leading/trailing underscores are trimmed. Output is capped at 64 bytes (common
//! `MySQL` identifier limit). Callers should treat an empty result as “nothing usable” and pick a
//! fallback.

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MANIFEST_REL_PATH: &str = ".coherence/project.toml";
const PROJECT_FILENAME: &str = "project.toml";

/// Latest manifest [`ProjectManifest::version`] when authoring optional `project_hash`.
pub const CURRENT_MANIFEST_SCHEMA_VERSION: u32 = 2;

/// Maximum length for a sanitized Dolt/MySQL database name segment.
pub const DOLT_DB_NAME_MAX_LEN: usize = 64;

/// Environment variable name for the logical Coherence deployment tier (`dev` / `test` / `prod`).
pub const COHERENCE_ENV_VAR: &str = "COHERENCE_ENV";

/// Logical deployment tier used when composing normalized catalog names ([`effective_dolt_catalog_name`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoherenceEnv {
    Dev,
    Test,
    Prod,
}

impl CoherenceEnv {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            CoherenceEnv::Dev => "dev",
            CoherenceEnv::Test => "test",
            CoherenceEnv::Prod => "prod",
        }
    }
}

impl FromStr for CoherenceEnv {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dev" => Ok(Self::Dev),
            "test" => Ok(Self::Test),
            "prod" => Ok(Self::Prod),
            other => Err(format!(
                "{COHERENCE_ENV_VAR}: invalid value {other:?} (expected dev, test, or prod)"
            )),
        }
    }
}

/// Parse `COHERENCE_ENV`-style tier text.
///
/// Returns [`CoherenceEnv::Dev`] when `raw` is `None`, empty, or whitespace-only (default for
/// interactive work when the variable is unset).
pub fn parse_coherence_env(raw: Option<&str>) -> Result<CoherenceEnv, String> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(CoherenceEnv::Dev);
    };
    s.parse()
}

/// Read [`COHERENCE_ENV_VAR`] from the process environment.
///
/// Missing or empty/unset values default to [`CoherenceEnv::Dev`]. Non-empty invalid values return
/// [`Err`].
pub fn coherence_env_from_std_env() -> Result<CoherenceEnv, String> {
    match std::env::var_os(COHERENCE_ENV_VAR) {
        None => Ok(CoherenceEnv::Dev),
        Some(os) => {
            let cow = os.to_string_lossy();
            parse_coherence_env(Some(cow.as_ref()))
        }
    }
}

/// On-disk project manifest: identity and optional catalog binding (see design ADR / issue COREDB-2ft).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub version: u32,
    pub project_slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dolt_db_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frozen_git_toplevel: Option<String>,
    pub project_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dolt_mode: Option<String>,
}

/// Walk `start` and ancestors until a `.git` file or directory is found; return that directory.
#[must_use]
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

/// Absolute path to `.coherence/project.toml` under a repository root.
#[must_use]
pub fn coherence_manifest_path(repo_root: impl AsRef<Path>) -> PathBuf {
    manifest_path(repo_root)
}

/// Best-effort manifest: git root of [`std::env::current_dir`], then read `.coherence/project.toml`.
///
/// Returns `None` when cwd is not inside a git work tree, the manifest is missing, or parsing fails.
/// Callers that need to distinguish a corrupt manifest from a missing file should use
/// [`coherence_manifest_path`] and [`read_manifest`] directly.
#[must_use]
pub fn try_read_project_manifest_from_cwd() -> Option<ProjectManifest> {
    let cwd = std::env::current_dir().ok()?;
    let root = find_git_repo_root(cwd)?;
    read_manifest(&root).ok()
}

fn validate_manifest(manifest: &ProjectManifest) -> Result<(), String> {
    if manifest.project_slug.trim().is_empty() {
        return Err("project_slug must be non-empty".to_string());
    }
    if let Some(ref h) = manifest.project_hash {
        if manifest.version < CURRENT_MANIFEST_SCHEMA_VERSION {
            return Err(format!(
                "project_hash requires manifest version >= {CURRENT_MANIFEST_SCHEMA_VERSION}"
            ));
        }
        if h.trim().is_empty() {
            return Err("project_hash must be non-empty when set".to_string());
        }
    }
    Ok(())
}

/// Map arbitrary input to a lowercase ASCII `a-z0-9_` segment, max [`DOLT_DB_NAME_MAX_LEN`].
#[must_use]
pub fn sanitize_dolt_db_segment(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if let Some(c) = dolt_db_segment_char(ch) {
            if !push_dolt_db_segment_char(&mut out, c) {
                break;
            }
        } else {
            push_dolt_db_separator(&mut out);
        }
    }
    trim_segment_to_fit(&mut out, DOLT_DB_NAME_MAX_LEN);
    out
}

fn push_dolt_db_segment_char(out: &mut String, ch: char) -> bool {
    if should_skip_underscore(ch, out) {
        return true;
    }
    if out.len() >= DOLT_DB_NAME_MAX_LEN {
        return false;
    }
    out.push(ch);
    true
}

fn push_dolt_db_separator(out: &mut String) {
    if can_push_dolt_db_separator(out) {
        out.push('_');
    }
}

fn can_push_dolt_db_separator(out: &str) -> bool {
    !out.is_empty() && !out.ends_with('_') && out.len() < DOLT_DB_NAME_MAX_LEN
}

fn dolt_db_segment_char(ch: char) -> Option<char> {
    match ch {
        'A'..='Z' => Some(ch.to_ascii_lowercase()),
        'a'..='z' | '0'..='9' => Some(ch),
        '_' => Some('_'),
        _ => None,
    }
}

fn should_skip_underscore(ch: char, out: &str) -> bool {
    ch == '_' && (out.is_empty() || out.ends_with('_'))
}

/// First four lowercase hex digits of SHA-256 over UTF-8 bytes of `path_for_hash`
/// (use the same string you persist as `frozen_git_toplevel`, after trimming).
#[must_use]
pub fn short_hash_frozen_git_path(path_for_hash: &str) -> String {
    let trimmed = path_for_hash.trim();
    let mut hasher = Sha256::new();
    hasher.update(trimmed.as_bytes());
    let full = format!("{:x}", hasher.finalize());
    full[..4].to_string()
}

/// `sanitize_dolt_db_segment(project_slug) + '_' + short_hash(frozen_git_toplevel)`,
/// capped at [`DOLT_DB_NAME_MAX_LEN`] for MySQL/Dolt database names.
pub fn dolt_db_name_for_bind(
    project_slug: &str,
    frozen_git_toplevel: &str,
) -> Result<String, String> {
    let short = short_hash_frozen_git_path(frozen_git_toplevel);
    let mut base = sanitize_dolt_db_segment(project_slug);
    if base.is_empty() {
        return Err(
            "project_slug sanitizes to an empty database name segment; use letters, digits, or underscores"
                .to_string(),
        );
    }
    let suffix = format!("_{short}");
    let max_base = DOLT_DB_NAME_MAX_LEN.saturating_sub(suffix.len());
    trim_segment_to_fit(&mut base, max_base);
    if base.is_empty() {
        return Err(
            "project_slug is too long to fit a stable dolt_db_name (max 64 characters)".to_string(),
        );
    }
    let name = format!("{base}{suffix}");
    debug_assert!(name.len() <= DOLT_DB_NAME_MAX_LEN);
    Ok(name)
}

/// Normalized Dolt/MySQL database identifier from slug, optional hash segment, and env tier.
///
/// Formula: sanitized **slug** `+` optional `_` **hash** `+` `_` **env** (`dev` / `test` / `prod`),
/// capped at [`DOLT_DB_NAME_MAX_LEN`]. See module docs.
pub fn effective_dolt_catalog_name(
    project_slug: &str,
    project_hash: Option<&str>,
    env: CoherenceEnv,
) -> Result<String, String> {
    let slug_full = sanitize_dolt_db_segment(project_slug);
    if slug_full.is_empty() {
        return Err(
            "project_slug sanitizes to an empty database name segment; use letters, digits, or underscores"
                .to_string(),
        );
    }

    let env_seg = env.as_str();
    let hash = project_hash
        .map(|h| sanitize_dolt_db_segment(h.trim()))
        .filter(|s| !s.is_empty());

    if let Some(ref h) = hash {
        if let Some(name) = try_compose_catalog(&slug_full, Some(h), env_seg) {
            return Ok(name);
        }
    }

    try_compose_catalog(&slug_full, None, env_seg).ok_or_else(|| {
        format!(
            "{COHERENCE_ENV_VAR}: project_slug is too long to fit a stable catalog name (max 64 characters)"
        )
    })
}

fn try_compose_catalog(slug_full: &str, hash: Option<&str>, env_seg: &str) -> Option<String> {
    let suffix = catalog_suffix(hash, env_seg);
    if suffix.len() > DOLT_DB_NAME_MAX_LEN {
        return None;
    }
    let max_base = DOLT_DB_NAME_MAX_LEN.saturating_sub(suffix.len());
    let mut base = slug_full.to_string();
    trim_segment_to_fit(&mut base, max_base);
    if base.is_empty() {
        return None;
    }
    let name = format!("{base}{suffix}");
    debug_assert!(name.len() <= DOLT_DB_NAME_MAX_LEN);
    Some(name)
}

fn catalog_suffix(hash: Option<&str>, env_seg: &str) -> String {
    match hash {
        Some(h) => format!("_{h}_{env_seg}"),
        None => format!("_{env_seg}"),
    }
}

fn trim_segment_to_fit(segment: &mut String, max_len: usize) {
    if segment.len() > max_len {
        segment.truncate(max_len);
    }
    while segment.ends_with('_') {
        segment.pop();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::fs::File;
    use std::io::Write;

    use tempfile::TempDir;

    fn assert_round_trip(manifest: ProjectManifest) {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), &manifest).unwrap();
        let loaded = read_manifest(tmp.path()).unwrap();
        assert_eq!(loaded, manifest);
    }

    fn assert_write_fails(manifest: ProjectManifest) {
        let tmp = TempDir::new().unwrap();
        assert!(write_manifest(tmp.path(), &manifest).is_err());
    }

    #[test]
    fn round_trip_write_read() {
        assert_round_trip(ProjectManifest {
            version: 1,
            project_slug: "my-project".to_string(),
            dolt_db_name: Some("my_catalog".to_string()),
            frozen_git_toplevel: Some("/tmp/repo".to_string()),
            project_hash: None,
            dolt_mode: None,
        });
    }

    #[test]
    fn read_rejects_empty_slug() {
        assert_write_fails(ProjectManifest {
            version: 1,
            project_slug: "   ".to_string(),
            dolt_db_name: None,
            frozen_git_toplevel: None,
            project_hash: None,
            dolt_mode: None,
        });
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

    #[test]
    fn short_hash_strips_whitespace_consistently() {
        let a = short_hash_frozen_git_path("/tmp/repo");
        let b = short_hash_frozen_git_path("  /tmp/repo  ");
        assert_eq!(a, b);
    }

    #[test]
    fn dolt_db_name_for_bind_formula_matches_slug_and_path_hash() {
        let n = dolt_db_name_for_bind("My-Project", "/workspace/foo").unwrap();
        let h = short_hash_frozen_git_path("/workspace/foo");
        assert_eq!(n, format!("my_project_{h}"));
        assert!(n.len() <= DOLT_DB_NAME_MAX_LEN);
    }

    #[test]
    fn round_trip_write_read_with_project_hash_v2() {
        assert_round_trip(ProjectManifest {
            version: CURRENT_MANIFEST_SCHEMA_VERSION,
            project_slug: "acme-core".to_string(),
            dolt_db_name: None,
            frozen_git_toplevel: None,
            project_hash: Some("a1b2".to_string()),
            dolt_mode: Some("user-scoped".to_string()),
        });
    }

    #[test]
    fn write_rejects_project_hash_on_schema_v1() {
        let tmp = TempDir::new().unwrap();
        let bad = ProjectManifest {
            version: 1,
            project_slug: "x".to_string(),
            dolt_db_name: None,
            frozen_git_toplevel: None,
            project_hash: Some("abcd".to_string()),
            dolt_mode: None,
        };
        let err = write_manifest(tmp.path(), &bad).unwrap_err();
        assert!(
            err.contains("version >= 2"),
            "expected version requirement error, got: {err}"
        );
    }

    #[test]
    fn parse_coherence_env_defaults_and_cases() {
        assert_eq!(parse_coherence_env(None).unwrap(), CoherenceEnv::Dev);
        assert_eq!(parse_coherence_env(Some("")).unwrap(), CoherenceEnv::Dev);
        assert_eq!(parse_coherence_env(Some("   ")).unwrap(), CoherenceEnv::Dev);
        assert_eq!(parse_coherence_env(Some("dev")).unwrap(), CoherenceEnv::Dev);
        assert_eq!(parse_coherence_env(Some("DEV")).unwrap(), CoherenceEnv::Dev);
        assert_eq!(
            parse_coherence_env(Some("test")).unwrap(),
            CoherenceEnv::Test
        );
        assert_eq!(
            parse_coherence_env(Some("Prod")).unwrap(),
            CoherenceEnv::Prod
        );
        assert!(parse_coherence_env(Some("staging")).is_err());
    }

    #[test]
    fn coherence_env_from_std_env_reads_variable() {
        let prev = std::env::var_os(COHERENCE_ENV_VAR);
        std::env::set_var(COHERENCE_ENV_VAR, "test");
        assert_eq!(coherence_env_from_std_env().unwrap(), CoherenceEnv::Test);
        std::env::set_var(COHERENCE_ENV_VAR, "");
        assert_eq!(coherence_env_from_std_env().unwrap(), CoherenceEnv::Dev);
        match prev {
            None => std::env::remove_var(COHERENCE_ENV_VAR),
            Some(v) => std::env::set_var(COHERENCE_ENV_VAR, v),
        }
    }

    #[test]
    fn effective_dolt_catalog_name_three_segments() {
        let n = effective_dolt_catalog_name("My-App", Some("cafe"), CoherenceEnv::Dev).unwrap();
        assert_eq!(n, "my_app_cafe_dev");
        assert!(n.len() <= DOLT_DB_NAME_MAX_LEN);
    }

    #[test]
    fn effective_dolt_catalog_name_skips_empty_hash() {
        let n = effective_dolt_catalog_name("svc", Some("   "), CoherenceEnv::Test).unwrap();
        assert_eq!(n, "svc_test");
    }

    #[test]
    fn effective_dolt_catalog_name_env_segments() {
        assert_eq!(
            effective_dolt_catalog_name("p", None, CoherenceEnv::Test).unwrap(),
            "p_test"
        );
        assert_eq!(
            effective_dolt_catalog_name("p", None, CoherenceEnv::Prod).unwrap(),
            "p_prod"
        );
    }

    #[test]
    fn effective_dolt_catalog_name_truncates_for_max_len() {
        let slug = "s".repeat(DOLT_DB_NAME_MAX_LEN);
        let n = effective_dolt_catalog_name(&slug, Some("hh"), CoherenceEnv::Prod).unwrap();
        assert_eq!(n.len(), DOLT_DB_NAME_MAX_LEN);
        assert!(n.ends_with("_hh_prod"), "got {n:?}");
    }

    #[test]
    fn effective_dolt_catalog_name_errors_empty_slug() {
        assert!(effective_dolt_catalog_name("@@@", Some("x"), CoherenceEnv::Dev).is_err());
    }
}
