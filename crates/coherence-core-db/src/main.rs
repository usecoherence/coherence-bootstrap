mod ac_code_link_store;
mod ac_verify;
mod cli;
mod commands;
mod db;
mod migrations;
mod models;
mod spec_store;

/// AC ↔ code location persistence for COREDB-12 and future CLI (keeps `cargo check` clean).
pub mod codeintel_repo {
    pub use crate::ac_code_link_store::{
        get_code_location, list_code_links_for_ac, put_ac_code_link, put_code_location,
    };
    pub use crate::ac_verify::{
        verify_acceptance_criterion, AcVerifyAcRunResult, AcVerifyLinkRunRecord, AcVerifyLinkStatus,
    };
    pub use crate::models::{
        AcCodeLink, AcCodeLinkWithLocation, AcCodeRelationKind, CodeLocation, CodeLocationKind,
    };
}

fn main() {
    let code = cli::run(std::env::args().collect());
    std::process::exit(code);
}
