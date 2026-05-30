//! Generic command-boundary test worlds for acceptance-criteria tests.
//!
//! The crate prepares disposable worlds and then executes AC checks as normal
//! shell commands inside those worlds. That command boundary is intentional:
//! Rust, Python, Node, Go, shell, or any other language can participate without
//! a language-specific adapter as long as the project can expose a verifier
//! command such as `cargo test`, `pytest`, `npm test`, or `./verify-ac.sh`.
//!
//! Current scope is local process execution plus service lifecycles such as
//! Dolt. Containers, recipe CLIs, and language-specific adapters are deferred
//! until the `World` / `Recipe` / `Service` API stabilizes.

#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::return_self_not_must_use
)]

pub mod dolt_world;
pub mod env_guard;
pub mod recipe;
pub mod scaffold;
pub mod service;
pub mod world;

pub use dolt_world::{DoltServer, DoltWorld};
pub use env_guard::EnvGuard;
pub use recipe::{E2eEnvironment, E2eRecipe};
pub use scaffold::Scaffold;
pub use service::{RunningService, Service, Services};
pub use world::{AcTest, CommandRequest, Evidence, VerificationResult, World, WorldRecipe};
