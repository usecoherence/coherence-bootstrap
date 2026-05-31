#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::type_complexity
)]

extern crate self as coherence_core_db;

pub mod ac_code_link_store;
pub mod ac_materialize_codeintel_ids;
pub mod ac_test_layout;
pub mod ac_verify;
pub mod cli;
pub mod commands;
pub mod db;
pub mod evidence_store;
pub mod migrations;
pub mod models;
pub mod project_manifest;
pub mod spec_store;
pub mod test_world_guard;

/// Codeintel persistence + verification helpers (`codeintel_*` tables).
pub mod codeintel_repo {
    pub use crate::ac_code_link_store::{
        get_code_location, list_code_links_for_ac, put_ac_code_link, put_code_location,
    };
    pub use crate::ac_verify::{
        verify_acceptance_criterion, verify_spec, AcVerifyAcRunResult, AcVerifyLinkRunRecord,
        AcVerifyLinkStatus, AcVerifyOverallStatus, VerifySpecRunResult,
    };
    pub use crate::models::{
        AcCodeLink, AcCodeLinkWithLocation, AcCodeRelationKind, CodeLocation, CodeLocationKind,
    };
}
