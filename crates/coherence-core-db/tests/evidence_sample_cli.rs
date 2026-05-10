use std::fs;
use std::process::Command;

#[test]
fn evidence_sample_cli_end_to_end() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let bin = env!("CARGO_BIN_EXE_coherence-core-db");
    let workspace = tmp.path().to_str().expect("utf8 temp path");
    let output = Command::new(bin)
        .args([
            "evidence-sample",
            "--workspace",
            workspace,
            "--run-id",
            "run-cli-e2e",
        ])
        .output()
        .expect("run evidence-sample");

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("retrieval:"),
        "expected retrieval instructions: {stdout}"
    );
    assert!(stdout.contains("large_payload_stays_outside_canonical_db"));

    let run_root = tmp
        .path()
        .join(".coherence")
        .join("runs")
        .join("run-cli-e2e");
    let meta = run_root.join("metadata");
    assert!(meta.join("run.json").is_file());
    assert!(meta.join("canonical-pointer.json").is_file());
    let obs = meta.join("observations").join("obs-sample-001.json");
    assert!(obs.is_file());
    let obs_raw = fs::read_to_string(&obs).expect("read observation");
    assert!(
        obs_raw.contains("plan-demo") && obs_raw.contains("AC-DEMO"),
        "observation should carry optional plan/ac ids in payload: {obs_raw}"
    );

    let ptr_path = meta.join("canonical-pointer.json");
    let ptr_raw = fs::read_to_string(&ptr_path).expect("read pointer stub");
    assert!(
        ptr_raw.len() < 2_000,
        "canonical stub must stay small (metadata only): {} bytes",
        ptr_raw.len()
    );
    assert!(
        ptr_raw.contains("artifacts/blobs/heavy-payload.bin"),
        "pointer should name artifact path under run root: {ptr_raw}"
    );

    let artifact = run_root.join("artifacts/blobs/heavy-payload.bin");
    let blob = fs::read(&artifact).expect("read artifact");
    assert_eq!(blob.len(), 1_048_576);
}
