#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::process::Command;

fn coherence_bin() -> String {
    env!("CARGO_BIN_EXE_coherence-core-db").to_string()
}

fn with_isolated_env(cmd: &mut Command) {
    cmd.env("COHERENCE_DB_PROFILE", "test");
    cmd.env("COHERENCE_ENV", "test");
    cmd.env_remove("DOLT_DB");
}

#[test]
fn verify_ac_requires_ac_id() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("verify-ac");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("verify-ac without id");
    assert!(
        !out.status.success(),
        "verify-ac without id should fail; got exit {}",
        out.status
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        combined.contains("usage") || combined.contains("--ac-id"),
        "error should mention usage or --ac-id; got: {combined}",
    );
}

#[test]
fn verify_ac_accepts_ac_id_positional() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("verify-ac").arg("NONEXISTENT-AC-1");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("verify-ac with positional id");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        combined.contains("OVERALL"),
        "verify-ac output should contain OVERALL line even for nonexistent AC; got: {combined}",
    );
}

#[test]
fn verify_ac_accepts_ac_id_flag() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("verify-ac").arg("--ac-id").arg("NONEXISTENT-AC-2");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("verify-ac with --ac-id flag");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        combined.contains("OVERALL"),
        "verify-ac output should contain OVERALL line even with --ac-id; got: {combined}",
    );
}

#[test]
fn verify_ac_output_contains_overall_line() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("verify-ac").arg("ANY-AC-FORMAT-TEST");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("verify-ac");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("OVERALL"),
        "verify-ac output should contain OVERALL line; got: {stdout}",
    );
}

#[test]
fn verify_ac_exits_zero_for_nonexistent_ac() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("verify-ac").arg("NONEXISTENT-AC-EXIT-TEST");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("verify-ac");
    assert!(
        out.status.success(),
        "verify-ac for nonexistent AC should exit 0 (no links to fail); got exit {}",
        out.status
    );
}
