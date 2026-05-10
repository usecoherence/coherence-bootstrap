pub fn run() -> i32 {
    println!("coherence-core-db doctor");
    println!("status: ok");
    println!("workflow_backend: local_stub");
    println!("orchestration_owner: external");
    println!(
        "canonical_db_policy: curated catalog only for reasoning state — tests/smoke refuse writes unless COHERENCE_DB_PROFILE=test (see coherence-core-db help, AGENTS.md)"
    );
    0
}
