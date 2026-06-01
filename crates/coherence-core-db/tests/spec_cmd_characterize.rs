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
fn spec_add_requires_slug_flag() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("spec").arg("add").arg("--title").arg("Test Spec");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("spec add without --slug");
    assert!(
        !out.status.success(),
        "spec add without --slug should fail; got exit {}",
        out.status
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("--slug") || err.contains("slug"),
        "error should mention --slug; got: {err}",
    );
}

#[test]
fn spec_add_requires_title_flag() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("spec").arg("add").arg("--slug").arg("test-slug");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("spec add without --title");
    assert!(
        !out.status.success(),
        "spec add without --title should fail; got exit {}",
        out.status
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("--title") || err.contains("title"),
        "error should mention --title; got: {err}",
    );
}

#[test]
fn spec_add_description_is_optional() {
    let slug = format!(
        "test-slug-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("spec")
        .arg("add")
        .arg("--slug")
        .arg(&slug)
        .arg("--title")
        .arg("Test Spec Without Description");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("spec add without --description");
    assert!(
        out.status.success(),
        "spec add without --description should succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("spec_id:"),
        "output should contain spec_id; got: {stdout}",
    );
    assert!(
        stdout.contains("slug:") && stdout.contains(&slug),
        "output should contain slug; got: {stdout}",
    );
}

#[test]
fn spec_add_returns_created_identity() {
    let slug = format!(
        "test-slug-ret-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("spec")
        .arg("add")
        .arg("--slug")
        .arg(&slug)
        .arg("--title")
        .arg("Identity Return Test");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("spec add");
    assert!(
        out.status.success(),
        "spec add should succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stdout.contains("spec_id:") && stdout.contains("slug:"),
        "spec add should print spec_id and slug to stdout; stdout: {stdout}; stderr: {stderr}",
    );
}

#[test]
fn spec_list_requires_no_args() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("spec").arg("list").arg("extra-arg");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("spec list with args");
    assert!(
        !out.status.success(),
        "spec list with extra args should fail; got exit {}",
        out.status
    );
}

#[test]
fn spec_show_requires_id() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("spec").arg("show");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("spec show without id");
    assert!(
        !out.status.success(),
        "spec show without id should fail; got exit {}",
        out.status
    );
}

#[test]
fn unknown_spec_subcommand_exits_nonzero() {
    let mut cmd = Command::new(&coherence_bin());
    cmd.arg("spec").arg("unknown-subcommand");
    with_isolated_env(&mut cmd);
    let out = cmd.output().expect("unknown spec subcommand");
    assert!(
        !out.status.success(),
        "unknown subcommand should exit nonzero; got {}",
        out.status
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("unknown spec subcommand"),
        "error should mention unknown subcommand; got: {err}",
    );
}
