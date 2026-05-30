//! AC: ac-product-tui-browser-project-env

#![allow(clippy::unwrap_used)]

#[path = "ac_support/project_env_selection.rs"]
mod project_env_selection;

#[test]
fn validates_project_env_selection() {
    project_env_selection::run_project_env_selection_spec_ac_e2e();
}
