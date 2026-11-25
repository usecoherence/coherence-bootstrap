//! Command surface and dispatch for the `coherence` binary.
//!
//! The root package also exists so tests materialized by `code materialize`
//! are real Cargo integration tests in this workspace.

pub mod cli;
pub mod legacy;

/// Parse arguments and dispatch. Returns a process exit code.
pub fn run(args: &[String]) -> i32 {
    cli::run(args)
}
