#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::process::Command;

fn coherence_bin() -> String {
    env!("CARGO_BIN_EXE_coherence-core-db").to_string()
}

fn with_isolated_env(cmd: &mut Command) {
    cmd.env("COHERENCE_DB_PROFILE", "test");
    cmd.env("COHERENCE_ENV", "test");
    cmd.env_remove("DOLT_DB");
    if std::env::var("DOLT_SOCKET").is_err() {
        cmd.env("DOLT_PORT", "33306");
    }
}

#[test]
fn verify_spec_requires_spec_id() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("verify-spec");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("verify-spec without id");
    assert!(
        !out.status.success(),
        "verify-spec without id should fail; got exit {}",
        out.status
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        combined.contains("usage") || combined.contains("--spec-id"),
        "error should mention usage or --spec-id; got: {combined}",
    );
}

#[test]
fn verify_spec_accepts_spec_id_positional() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("verify-spec").arg("NONEXISTENT-SPEC-1");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("verify-spec with positional id");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        !combined.is_empty(),
        "verify-spec should produce output (error for not-found is ok); got empty"
    );
}

#[test]
fn verify_spec_accepts_spec_id_flag() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("verify-spec")
        .arg("--spec-id")
        .arg("NONEXISTENT-SPEC-2");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("verify-spec with --spec-id flag");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        !combined.is_empty(),
        "verify-spec should produce output (error for not-found is ok); got empty"
    );
}

#[test]
fn verify_spec_exits_nonzero_for_nonexistent_spec() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("verify-spec").arg("NONEXISTENT-SPEC-EXIT-TEST");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("verify-spec");
    assert!(
        !out.status.success(),
        "verify-spec for nonexistent spec should exit nonzero; got exit {}",
        out.status
    );
}

#[test]
fn verify_spec_error_contains_spec_not_found() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("verify-spec").arg("NONEXISTENT-SPEC-ERROR-TEST");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("verify-spec");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("spec not found"),
        "error should mention 'spec not found'; got: {stderr}",
    );
}
