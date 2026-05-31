//! coherence-core-db binary: Milestone‑1 demonstrates one local Coherence-compatible DB carrying
//! spec metadata **and** codeintel linkage (`codeintel_*` tables), plus shell-based verification.
//! Canonical narrative (command flow, table owners, non-goals): see `AGENTS.md` § M1 module
//! ownership — do not duplicate that essay here; module comments summarize slice boundaries only.
//! 
#[cfg(test)]
mod ac_tests_materialize_integration;

fn main() {
    let code = coherence_core_db::cli::run(&std::env::args().collect::<Vec<_>>());
    std::process::exit(code);
}
