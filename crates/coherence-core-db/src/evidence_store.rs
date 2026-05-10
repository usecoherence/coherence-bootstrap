//! Managed per-run evidence store layout (ADR-0005).
//!
//! Run-scoped evidence lives under `.coherence/runs/<run-id>/` (see [`RunLayout`]) — not in the curated
//! canonical Dolt catalog. Heavy bytes stay on disk under `artifacts/`; metadata is JSON.
//!
//! # Canonical database boundary
//!
//! M1 `spec` / `codeintel` migrations do **not** add evidence tables. The canonical catalog must
//! never store large payload blobs for evidence — only pointer-style metadata (run id, relative
//! paths, hashes, summaries) when a future migration wires [`CanonicalEvidencePointer`] into SQL.
//! Until then, [`write_canonical_pointer_stub`] writes the same shape next to the run for demos
//! and tests (retrieval path without coupling to a runtime DB backend).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// `.coherence` at the workspace root.
pub const COHERENCE_DIR: &str = ".coherence";
/// Directory under `.coherence` holding per-run evidence roots.
pub const RUNS_ROOT_SEGMENT: &str = "runs";

/// Subdirectory of a run root for content-addressed or named artifact files (large payloads).
pub const ARTIFACTS_SEGMENT: &str = "artifacts";
/// Subdirectory for per-observation JSON records.
pub const OBSERVATIONS_SEGMENT: &str = "observations";

pub const RUN_MANIFEST_FILE: &str = "run.json";
/// File holding the pointer record shape we plan to persist in the canonical DB later (M0 stub).
pub const CANONICAL_POINTER_STUB_FILE: &str = "canonical-pointer.json";

/// Layout paths for `.coherence/runs/<run-id>/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLayout {
    pub workspace_root: PathBuf,
    pub run_id: String,
}

impl RunLayout {
    pub fn new(workspace_root: impl Into<PathBuf>, run_id: impl Into<String>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            run_id: run_id.into(),
        }
    }

    /// `.coherence/runs/<run-id>/`
    pub fn run_root(&self) -> PathBuf {
        self.workspace_root
            .join(COHERENCE_DIR)
            .join(RUNS_ROOT_SEGMENT)
            .join(&self.run_id)
    }

    pub fn artifacts_dir(&self) -> PathBuf {
        self.run_root().join(ARTIFACTS_SEGMENT)
    }

    pub fn observations_dir(&self) -> PathBuf {
        self.run_root().join(OBSERVATIONS_SEGMENT)
    }

    pub fn run_manifest_path(&self) -> PathBuf {
        self.run_root().join(RUN_MANIFEST_FILE)
    }

    pub fn canonical_pointer_stub_path(&self) -> PathBuf {
        self.run_root().join(CANONICAL_POINTER_STUB_FILE)
    }
}

/// Top-level metadata for a run directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    pub schema_version: u32,
    pub run_id: String,
    /// RFC 3339 timestamp when the run directory was created.
    pub created_at: String,
}

/// Pointer to a large payload kept outside any tabular / canonical row payload column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadPointer {
    pub summary: String,
    pub sha256_hex: String,
    /// Path relative to the run root (e.g. `artifacts/large.bin`).
    pub artifact_relpath: String,
}

/// Typed envelope for a single observation (JSON on disk; future adapter contract).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationRecord {
    pub run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ac_id: Option<String>,
    pub observation_id: String,
    pub object_kind: String,
    pub object_id: String,
    pub payload: PayloadPointer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redaction_policy_id: Option<String>,
}

/// Metadata only — the shape intended for a future canonical row; **no inline blob field**.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalEvidencePointer {
    pub run_id: String,
    /// Relative to workspace root, POSIX-style for logs (e.g. `.coherence/runs/abc`).
    pub evidence_root_relpath: String,
    pub observation_id: String,
    /// Path relative to the run root (join with evidence dir to open the artifact).
    pub artifact_relpath_from_run_root: String,
    pub payload_sha256_hex: String,
    pub payload_summary: String,
}

/// Ensure run root, `artifacts/`, and `observations/` exist.
pub fn ensure_run_directories(layout: &RunLayout) -> Result<(), String> {
    fs::create_dir_all(layout.artifacts_dir()).map_err(|e| format!("create artifacts dir: {e}"))?;
    fs::create_dir_all(layout.observations_dir())
        .map_err(|e| format!("create observations dir: {e}"))?;
    Ok(())
}

pub fn write_run_manifest(layout: &RunLayout, manifest: &RunManifest) -> Result<(), String> {
    fs::create_dir_all(layout.run_root()).map_err(|e| format!("create run root: {e}"))?;
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("serialize run manifest: {e}"))?;
    fs::write(layout.run_manifest_path(), json).map_err(|e| format!("write run.json: {e}"))?;
    Ok(())
}

pub fn write_bytes_under_artifacts(
    layout: &RunLayout,
    rel_under_artifacts: &str,
    bytes: &[u8],
) -> Result<String, String> {
    ensure_run_directories(layout)?;
    let dest = layout.artifacts_dir().join(rel_under_artifacts);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create artifact parent: {e}"))?;
    }
    fs::write(&dest, bytes).map_err(|e| format!("write artifact: {e}"))?;
    Ok(sha256_hex(bytes))
}

pub fn write_observation(layout: &RunLayout, record: &ObservationRecord) -> Result<(), String> {
    ensure_run_directories(layout)?;
    let path = layout
        .observations_dir()
        .join(format!("{}.json", record.observation_id));
    let json =
        serde_json::to_string_pretty(record).map_err(|e| format!("serialize observation: {e}"))?;
    fs::write(path, json).map_err(|e| format!("write observation: {e}"))?;
    Ok(())
}

pub fn write_canonical_pointer_stub(
    layout: &RunLayout,
    pointer: &CanonicalEvidencePointer,
) -> Result<(), String> {
    fs::create_dir_all(layout.run_root()).map_err(|e| format!("create run root: {e}"))?;
    let json = serde_json::to_string_pretty(pointer)
        .map_err(|e| format!("serialize canonical pointer: {e}"))?;
    fs::write(layout.canonical_pointer_stub_path(), json)
        .map_err(|e| format!("write canonical-pointer.json: {e}"))?;
    Ok(())
}

/// Resolve filesystem path for the artifact referenced by canonical metadata (and observation).
pub fn resolve_artifact_path(workspace: &Path, pointer: &CanonicalEvidencePointer) -> PathBuf {
    workspace
        .join(&pointer.evidence_root_relpath)
        .join(&pointer.artifact_relpath_from_run_root)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// End-to-end sample: manifest, multi-byte artifact outside DB, observation envelope, canonical stub.
pub fn bootstrap_sample_run(
    workspace_root: impl AsRef<Path>,
    run_id: impl AsRef<str>,
) -> Result<CanonicalEvidencePointer, String> {
    let layout = RunLayout::new(
        workspace_root.as_ref().to_path_buf(),
        run_id.as_ref().to_owned(),
    );
    let manifest = RunManifest {
        schema_version: 1,
        run_id: layout.run_id.clone(),
        created_at: created_at_label(),
    };

    write_run_manifest(&layout, &manifest)?;

    let large_body: Vec<u8> = vec![0xABu8; 1_048_576];
    let artifact_under_artifacts = Path::new("blobs").join("heavy-payload.bin");
    let artifact_under_artifacts_str = artifact_under_artifacts
        .to_string_lossy()
        .replace('\\', "/");
    let hash = write_bytes_under_artifacts(&layout, &artifact_under_artifacts_str, &large_body)?;

    let artifact_relpath_from_run_root = Path::new(ARTIFACTS_SEGMENT)
        .join(&artifact_under_artifacts)
        .to_string_lossy()
        .replace('\\', "/");

    let observation_id = "obs-sample-001".to_string();
    let obs = ObservationRecord {
        run_id: layout.run_id.clone(),
        plan_id: Some("plan-demo".to_owned()),
        ac_id: Some("AC-DEMO".to_owned()),
        observation_id: observation_id.clone(),
        object_kind: "http_response".to_owned(),
        object_id: "/api/users?page=1".to_owned(),
        payload: PayloadPointer {
            summary: "Large JSON blob redacted/truncated; full body in artifact".to_owned(),
            sha256_hex: hash.clone(),
            artifact_relpath: artifact_relpath_from_run_root.clone(),
        },
        redaction_policy_id: Some("redact-email-v1".to_owned()),
    };
    write_observation(&layout, &obs)?;

    let evidence_root_relpath = compute_evidence_root_relpath(&layout)?;

    let pointer = CanonicalEvidencePointer {
        run_id: layout.run_id.clone(),
        evidence_root_relpath,
        observation_id,
        artifact_relpath_from_run_root: artifact_relpath_from_run_root.clone(),
        payload_sha256_hex: hash,
        payload_summary: obs.payload.summary.clone(),
    };

    write_canonical_pointer_stub(&layout, &pointer)?;

    Ok(pointer)
}

fn compute_evidence_root_relpath(layout: &RunLayout) -> Result<String, String> {
    let run_root = layout.run_root();
    let rel = run_root
        .strip_prefix(&layout.workspace_root)
        .map_err(|_| "run root must be under workspace_root".to_owned())?;
    Ok(rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn created_at_label() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    format!("unix_timestamp_seconds={secs}")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_sample_keeps_payload_on_disk_and_pointer_has_no_blob() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let run_id = "run-test-1";
        let ptr = bootstrap_sample_run(tmp.path(), run_id).expect("bootstrap");

        let artifact = resolve_artifact_path(tmp.path(), &ptr);
        let bytes = fs::read(&artifact).expect("read artifact");
        assert_eq!(bytes.len(), 1_048_576);
        assert_eq!(sha256_hex(&bytes), ptr.payload_sha256_hex);

        let stub_raw = fs::read_to_string(
            tmp.path()
                .join(ptr.evidence_root_relpath)
                .join("canonical-pointer.json"),
        )
        .expect("read stub");
        assert!(
            !stub_raw.contains("ABABAB"),
            "stub must not embed raw payload bytes"
        );
        assert!(
            stub_raw.len() < 800,
            "stub should be metadata only: {} bytes",
            stub_raw.len()
        );
    }
}
