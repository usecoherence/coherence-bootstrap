use std::process::Command;

#[test]
fn help_prints_workflow_entrypoint() {
    let bin = env!("CARGO_BIN_EXE_coherence-core-db");
    let output = Command::new(bin).arg("help").output().expect("run help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("make tool help"));
    assert!(stdout.contains("verify-ac"));
    assert!(stdout.contains("verify-spec"));
    assert!(
        stdout.contains("COHERENCE_DB_PROFILE=test"),
        "help should describe isolated profile policy: {stdout}"
    );
    assert!(
        stdout.contains("Canonical repository database"),
        "help should introduce canonical DB policy: {stdout}"
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
        stdout.contains("COHERENCE_DB_PROFILE=test"),
        "doctor should name isolated profile env: {stdout}"
    );
}
