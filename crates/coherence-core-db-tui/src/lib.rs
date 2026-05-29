#![allow(
    clippy::assigning_clones,
    clippy::expect_used,
    clippy::format_push_string,
    clippy::ignored_unit_patterns,
    clippy::implicit_clone,
    clippy::let_unit_value,
    clippy::manual_let_else,
    clippy::manual_string_new,
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::match_single_binding,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::needless_borrow,
    clippy::needless_pass_by_value,
    clippy::redundant_clone,
    clippy::single_match_else,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::wildcard_enum_match_arm,
)]

pub mod action;
pub mod app;
pub mod edit;
pub mod effects;
pub mod project_discovery;
pub mod repository;
pub mod theme;
pub mod tree;
pub mod update;
pub mod ui;

pub use action::{key_to_action, AppAction};
pub use app::{AppState, Screen};
pub use edit::Draft;
pub use effects::Effect;
pub use update::update;
