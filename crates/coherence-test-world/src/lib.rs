#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::return_self_not_must_use
)]

pub mod dolt_world;
pub mod scaffold;
pub mod world;

pub use dolt_world::DoltWorld;
pub use scaffold::Scaffold;
pub use world::{AcTest, Evidence, VerificationResult, World, WorldRecipe};
