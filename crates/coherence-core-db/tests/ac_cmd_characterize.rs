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
fn ac_add_requires_spec_id() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("ac").arg("add").arg("--title").arg("Test AC");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("ac add without --spec-id");
    assert!(
        !out.status.success(),
        "ac add without --spec-id should fail; got exit {}",
        out.status
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("--spec-id") || err.contains("spec-id"),
        "error should mention --spec-id; got: {err}",
    );
}

#[test]
fn ac_add_requires_title() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("ac").arg("add").arg("--spec-id").arg("TEST-SPEC-1");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("ac add without --title");
    assert!(
        !out.status.success(),
        "ac add without --title should fail; got exit {}",
        out.status
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("--title") || err.contains("title"),
        "error should mention --title; got: {err}",
    );
}

#[test]
fn ac_add_validates_spec_exists_before_creating() {
    let nonexistent_spec = format!(
        "AC-ADD-SPEC-MISSING-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("ac")
        .arg("add")
        .arg("--spec-id")
        .arg(&nonexistent_spec)
        .arg("--title")
        .arg("AC for Missing Spec");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("ac add");
    assert!(
        !out.status.success(),
        "ac add should fail when spec does not exist; got exit {}",
        out.status
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("spec not found"),
        "error should mention 'spec not found'; got: {stderr}",
    );
}

#[test]
fn ac_list_requires_spec_id() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("ac").arg("list");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("ac list without --spec-id");
    assert!(
        !out.status.success(),
        "ac list without --spec-id should fail; got exit {}",
        out.status
    );
}

#[test]
fn ac_list_rejects_extra_args() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("ac")
        .arg("list")
        .arg("--spec-id")
        .arg("TEST-SPEC-1")
        .arg("extra-arg");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("ac list with extra args");
    assert!(
        !out.status.success(),
        "ac list with extra args should fail; got exit {}",
        out.status
    );
}

#[test]
fn ac_show_requires_id() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("ac").arg("show");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("ac show without id");
    assert!(
        !out.status.success(),
        "ac show without id should fail; got exit {}",
        out.status
    );
}

#[test]
fn unknown_ac_subcommand_exits_nonzero() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("ac").arg("unknown-subcommand");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("unknown ac subcommand");
    assert!(
        !out.status.success(),
        "unknown subcommand should exit nonzero; got {}",
        out.status
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("unknown ac subcommand"),
        "error should mention unknown subcommand; got: {err}",
    );
}
