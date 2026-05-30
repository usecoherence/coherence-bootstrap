#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::return_self_not_must_use
)]

pub mod dolt_world;
pub mod recipe;
pub mod scaffold;
pub mod world;

pub use dolt_world::{DoltServer, DoltWorld};
pub use recipe::{E2eEnvironment, E2eRecipe};
pub use scaffold::Scaffold;
pub use world::{AcTest, CommandRequest, Evidence, VerificationResult, World, WorldRecipe};
