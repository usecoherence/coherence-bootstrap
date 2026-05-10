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
}
