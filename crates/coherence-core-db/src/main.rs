//! coherence-core-db binary: Milestone‑1 demonstrates one local Coherence-compatible DB carrying
//! spec metadata **and** codeintel linkage (`codeintel_*` tables), plus shell-based verification.
//! Canonical narrative (command flow, table owners, non-goals): see `AGENTS.md` § M1 module
//! ownership — do not duplicate that essay here; module comments summarize slice boundaries only.
//!
mod cli;
mod commands;
#[cfg(test)]
mod ac_tests_materialize_integration;

/// **Codeintel persistence + verification helpers** (`codeintel_*` tables): put/list locations &
/// AC links (`verify-*` consumes `verified_by` links). CLI writes for locations/links may land
/// later; M1 callers use these APIs alongside `spec …` / `ac …`.
pub mod codeintel_repo {
    pub use coherence_core_db::ac_code_link_store::{
        get_code_location, list_code_links_for_ac, put_ac_code_link, put_code_location,
    };
    pub use coherence_core_db::ac_verify::{
        verify_acceptance_criterion, verify_spec, AcVerifyAcRunResult, AcVerifyLinkRunRecord,
        AcVerifyLinkStatus, AcVerifyOverallStatus, VerifySpecRunResult,
    };
    pub use coherence_core_db::models::{
        AcCodeLink, AcCodeLinkWithLocation, AcCodeRelationKind, CodeLocation, CodeLocationKind,
    };
}

fn main() {
    let code = cli::run(&std::env::args().collect::<Vec<_>>());
    std::process::exit(code);
}
