#![allow(clippy::unwrap_used, clippy::expect_used, clippy::map_unwrap_or)]

use std::process::Command;

use tempfile::TempDir;

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[test]
fn help_prints_workflow_entrypoint() {
    let bin = env!("CARGO_BIN_EXE_coherence-core-db");
    let output = Command::new(bin).arg("help").output().expect("run help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("make tool help"));
    assert!(
        stdout.contains("make tool bootstrap"),
        "help should list first-time bootstrap: {stdout}"
    );
    assert!(stdout.contains("verify-ac"));
    assert!(stdout.contains("verify-spec"));
    assert!(stdout.contains("ac-tests"));
    assert!(
        stdout.contains("check-rust") && stdout.contains("materialize-rust"),
        "help should describe ac-tests subcommands: {stdout}"
    );
    assert!(stdout.contains("evidence-sample"));
    assert!(
        stdout.contains("COHERENCE_DB_PROFILE=test"),
        "help should describe isolated profile policy: {stdout}"
    );
    assert!(
        stdout.contains("Canonical repository database"),
        "help should introduce canonical DB policy: {stdout}"
    );
    assert!(
        stdout.contains("project"),
        "help should list project command: {stdout}"
    );
    assert!(
        stdout.contains("project init"),
        "help should describe project init: {stdout}"
    );
    assert!(
        stdout.contains("catalog-preflight"),
        "help should mention project catalog-preflight: {stdout}"
    );
    assert!(
        stdout.contains("project reset"),
        "help should mention project reset repair path: {stdout}"
    );
    assert!(
        stdout.contains("Project identity and manifest lifecycle"),
        "help should point at AGENTS.md subsection: {stdout}"
    );
    assert!(
        stdout.contains("COHERENCE_ENV (dev|test|prod"),
        "help should summarize COHERENCE_ENV one-liner: {stdout}"
    );
}

#[test]
fn doctor_reports_local_stub_backend() {
    let bin = env!("CARGO_BIN_EXE_coherence-core-db");
    let output = Command::new(bin)
        .arg("doctor")
        .output()
        .expect("run doctor");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("workflow_backend: local_stub"));
    assert!(stdout.contains("orchestration_owner: external"));
    assert!(
        stdout.contains("canonical_db_policy"),
        "doctor should surface canonical DB policy: {stdout}"
    );
    assert!(
        stdout.contains("managed_evidence"),
        "doctor should mention managed evidence (ADR-0005): {stdout}"
    );
    assert!(
        stdout.contains("COHERENCE_DB_PROFILE=test"),
        "doctor should name isolated profile env: {stdout}"
    );
    assert!(
        stdout.contains("COHERENCE_ENV:"),
        "doctor should report coherence env tier: {stdout}"
    );
    assert!(
        stdout.contains("effective_catalog_without_DOLT_DB_override:"),
        "doctor should report hypothetical catalog sans DOLT_DB: {stdout}"
    );
    assert!(
        stdout.contains("manifest_catalog_complete_for_connect_preflight:"),
        "doctor should report manifest completeness for guarded connect: {stdout}"
    );
    assert!(
        stdout.contains("git_root_found:")
            && stdout.contains("manifest_present:")
            && stdout.contains("manifest_path:")
            && stdout.contains("dolt_db_name_manifest:")
            && stdout.contains("env_DOLT_DB_override_active:"),
        "doctor should report manifest snapshot lines: {stdout}"
    );
    assert!(
        stdout.contains("project_hash_manifest:"),
        "doctor should summarize project_hash presence: {stdout}"
    );
}

#[test]
fn project_init_writes_manifest_once_and_force_rebind() {
    if !git_available() {
        eprintln!("skip project_init_writes_manifest_once_and_force_rebind: git not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    assert!(Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .status()
        .expect("git init")
        .success());

    let bin = env!("CARGO_BIN_EXE_coherence-core-db");
    let out = Command::new(bin)
        .args(["project", "init", "--slug", "myapp"])
        .current_dir(tmp.path())
        .output()
        .expect("project init");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let manifest_path = tmp.path().join(".coherence/project.toml");
    let raw = std::fs::read_to_string(&manifest_path).expect("manifest");
    assert!(raw.contains("project_slug = \"myapp\""));
    assert!(raw.contains("dolt_db_name = \"myapp_"));
    assert!(raw.contains("frozen_git_toplevel = "));
    assert!(
        raw.contains("project_hash = \""),
        "first init should persist project_hash; got:\n{raw}"
    );

    let canon = tmp.path().canonicalize().unwrap();
    let path_str = canon.to_str().expect("utf8 path");
    assert!(
        raw.contains(path_str),
        "manifest should record git toplevel path; got:\n{raw}"
    );

    let again = Command::new(bin)
        .args(["project", "init"])
        .current_dir(tmp.path())
        .output()
        .expect("second init");
    assert!(
        again.status.success(),
        "second init should be idempotent; stderr: {}",
        String::from_utf8_lossy(&again.stderr)
    );
    let again_raw = std::fs::read_to_string(&manifest_path).expect("manifest after second init");
    assert_eq!(
        again_raw, raw,
        "idempotent project init must not rewrite manifest"
    );

    let force = Command::new(bin)
        .args(["project", "init", "--force-rebind"])
        .current_dir(tmp.path())
        .output()
        .expect("force rebind");
    assert!(
        force.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&force.stderr)
    );
    let after_force = std::fs::read_to_string(&manifest_path).expect("manifest after force");
    assert!(
        after_force.contains("project_hash = \""),
        "--force-rebind should keep a bound project_hash; got:\n{after_force}"
    );
}

#[test]
fn project_init_migrates_legacy_manifest_with_dolt_and_frozen_but_no_hash() {
    if !git_available() {
        eprintln!(
            "skip project_init_migrates_legacy_manifest_with_dolt_and_frozen_but_no_hash: git not available"
        );
        return;
    }

    let tmp = TempDir::new().unwrap();
    assert!(Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .status()
        .expect("git init")
        .success());

    let canon = tmp.path().canonicalize().unwrap();
    let path_str = canon.to_str().expect("utf8 path");

    let coherence_dir = tmp.path().join(".coherence");
    std::fs::create_dir_all(&coherence_dir).unwrap();
    let manifest_path = coherence_dir.join("project.toml");
    std::fs::write(
        &manifest_path,
        format!(
            r#"version = 1
project_slug = "legacyapp"
dolt_db_name = "legacyapp_deadbeef"
frozen_git_toplevel = "{path_str}"
"#,
            path_str = path_str.replace('\\', "\\\\").replace('"', "\\\"")
        ),
    )
    .expect("write legacy manifest");

    let bin = env!("CARGO_BIN_EXE_coherence-core-db");
    let out = Command::new(bin)
        .args(["project", "init"])
        .current_dir(tmp.path())
        .output()
        .expect("project init");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let raw = std::fs::read_to_string(&manifest_path).expect("manifest");
    assert!(
        raw.contains("project_hash = \""),
        "migration should add project_hash; got:\n{raw}"
    );
}

#[test]
fn project_init_errors_when_legacy_dolt_without_frozen_or_hash() {
    if !git_available() {
        eprintln!(
            "skip project_init_errors_when_legacy_dolt_without_frozen_or_hash: git not available"
        );
        return;
    }

    let tmp = TempDir::new().unwrap();
    assert!(Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .status()
        .expect("git init")
        .success());

    let coherence_dir = tmp.path().join(".coherence");
    std::fs::create_dir_all(&coherence_dir).unwrap();
    let manifest_path = coherence_dir.join("project.toml");
    std::fs::write(
        &manifest_path,
        r#"version = 1
project_slug = "orphan"
dolt_db_name = "orphan_abcd"
"#,
    )
    .expect("write broken legacy manifest");

    let bin = env!("CARGO_BIN_EXE_coherence-core-db");
    let out = Command::new(bin)
        .args(["project", "init"])
        .current_dir(tmp.path())
        .output()
        .expect("project init");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("frozen_git_toplevel") && err.contains("force-rebind"),
        "expected migration error; stderr: {err}"
    );
}

#[test]
fn project_reset_errors_without_manifest_file() {
    if !git_available() {
        eprintln!("skip project_reset_errors_without_manifest_file: git not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    assert!(Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .status()
        .expect("git init")
        .success());

    let bin = env!("CARGO_BIN_EXE_coherence-core-db");
    let out = Command::new(bin)
        .args(["project", "reset"])
        .current_dir(tmp.path())
        .env_remove("DOLT_DB")
        .env_remove("COHERENCE_ENV")
        .env_remove("COHERENCE_PROJECT_SLUG")
        .env_remove("COHERENCE_DB_PROFILE")
        .output()
        .expect("project reset");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("project reset:") && err.contains(".coherence/project.toml"),
        "expected manifest read error; stderr: {err}"
    );
}

#[test]
fn db_ping_manifest_preflight_errors_in_git_repo_without_manifest_file() {
    if !git_available() {
        eprintln!(
            "skip db_ping_manifest_preflight_errors_in_git_repo_without_manifest_file: git not available"
        );
        return;
    }

    let tmp = TempDir::new().unwrap();
    assert!(Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .status()
        .expect("git init")
        .success());

    let bin = env!("CARGO_BIN_EXE_coherence-core-db");
    let out = Command::new(bin)
        .current_dir(tmp.path())
        .arg("db-ping")
        .env_remove("DOLT_DB")
        .env_remove("COHERENCE_ENV")
        .env_remove("COHERENCE_PROJECT_SLUG")
        .env_remove("COHERENCE_DB_PROFILE")
        .output()
        .expect("db-ping");
    assert!(
        !out.status.success(),
        "db-ping should refuse before socket/tcp when manifest is missing\nstderr:{}\nstdout:{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        combined.contains("db-ping:") || combined.contains("missing project manifest"),
        "expected manifest preflight error; got:{combined}",
    );
    assert!(
        combined.contains("project init") || combined.contains("--slug"),
        "expected remediation hint in output; got:{combined}",
    );
}
