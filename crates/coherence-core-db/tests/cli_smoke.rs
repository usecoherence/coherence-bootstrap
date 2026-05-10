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
        stdout.contains("Project identity and manifest lifecycle"),
        "help should point at AGENTS.md subsection: {stdout}"
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
        stdout.contains("git_root_found:")
            && stdout.contains("manifest_present:")
            && stdout.contains("manifest_path:")
            && stdout.contains("dolt_db_name_manifest:")
            && stdout.contains("env_DOLT_DB_override_active:"),
        "doctor should report manifest snapshot lines: {stdout}"
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
        !again.status.success(),
        "second init without --force-rebind should fail"
    );
    let err = String::from_utf8_lossy(&again.stderr);
    assert!(
        err.contains("dolt_db_name") && err.contains("force-rebind"),
        "stderr: {err}"
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
}
