# Commands: evidence — `evidence-sample`

**Classification:** observed — `evidence_sample_cmd.rs`, `evidence_store` module.

## `evidence-sample`

**Public intent:** Demonstrate ADR-0005-style per-run artifact layout: create `.coherence/runs/<run-id>/` under a chosen workspace with sample payload and pointer metadata.

**Inputs:** `--workspace <dir>` optional (defaults `current_dir`); `--run-id` optional (defaults UUID-based id).

**World:** writable filesystem under workspace; does not require Dolt for the happy path (uses evidence store helpers).

**Observable effects:** directories/files under `.coherence/runs/…`; stdout report with paths and checksum pointer.

**Failure modes:** parse errors, IO errors → stderr `evidence-sample: failed`, exit **1**.

**Relation to verification:** optional hooks in `verify-ac` when evidence env vars set — documented at verification layer in code; CLI surface here is the **demo** command only.
