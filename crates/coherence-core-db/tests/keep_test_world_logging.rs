//! Integration: `scripts/with-isolated-test-profile` stderr marker on failure + keep (COREDB-44l).
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn run_wrapper(args: &[&str], extra_env: &[(&str, &str)]) -> (i32, String) {
    let root = repo_root();
    let script = root.join("scripts/with-isolated-test-profile");
    let mut cmd = Command::new("bash");
    cmd.current_dir(&root)
        .env("COHERENCE_KEEP_TEST_WORLD", "1")
        .env_remove("DOLT_PASSWORD")
        .arg(&script);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("spawn with-isolated-test-profile");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let code = out.status.code().unwrap_or(-1);
    (code, stderr)
}

#[test]
fn failure_with_keep_prints_preservation_marker_and_db_line() {
    let (code, stderr) = run_wrapper(&["false"], &[]);
    assert_ne!(code, 0, "expected child failure");
    assert!(
        stderr.contains("COHERENCE_PRESERVED_TEST_WORLD: v1"),
        "stderr missing marker line: {stderr:?}"
    );
    assert!(
        stderr.contains("COHERENCE_PRESERVED_TEST_WORLD: DOLT_DB=coherence_test_"),
        "stderr missing DOLT_DB line: {stderr:?}"
    );
    assert!(
        stderr.contains("COHERENCE_PRESERVED_TEST_WORLD: DOLT_SOCKET="),
        "stderr missing socket line: {stderr:?}"
    );
    assert!(
        stderr.contains("reason=child_failed"),
        "stderr missing failure reason: {stderr:?}"
    );
}

#[test]
fn failure_with_keep_includes_run_id_when_set() {
    let (code, stderr) = run_wrapper(
        &["false"],
        &[("COHERENCE_ISOLATED_TEST_RUN_ID", "test-run-xyz")],
    );
    assert_ne!(code, 0);
    assert!(
        stderr.contains("COHERENCE_PRESERVED_TEST_WORLD: RUN_ID=test-run-xyz"),
        "stderr missing RUN_ID: {:?}",
        stderr
    );
}

#[test]
fn success_with_keep_is_quiet_without_verbose() {
    let (code, stderr) = run_wrapper(&["true"], &[]);
    assert_eq!(code, 0);
    assert!(
        !stderr.contains("COHERENCE_PRESERVED_TEST_WORLD:"),
        "unexpected preservation block on quiet success: {stderr:?}"
    );
}

#[test]
fn success_with_keep_prints_block_when_verbose() {
    let (code, stderr) = run_wrapper(&["true"], &[("COHERENCE_ISOLATED_TEST_VERBOSE", "1")]);
    assert_eq!(code, 0);
    assert!(
        stderr.contains("COHERENCE_PRESERVED_TEST_WORLD: v1"),
        "expected verbose success block: {stderr:?}"
    );
    assert!(
        stderr.contains("reason=success_keep"),
        "expected success_keep reason: {stderr:?}"
    );
}
