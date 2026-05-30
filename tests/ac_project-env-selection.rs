//! AC: ac-product-tui-browser-project-env

#[path = "ac_support/project_env_selection.rs"]
mod project_env_selection;

use project_env_selection::{
    assert_that, ProjectEnvSelectionWorld, PROJECT_ENV_AC_ID, PROJECT_ENV_SPEC_ID,
};

#[test]
fn validates_project_env_selection() -> Result<(), String> {
    let world = ProjectEnvSelectionWorld::builder()
        .with_dev_spec("dev-only-spec")
        .with_test_project_env_ac()
        .build()?;

    let mut app = world.open_tui()?;

    world
        .drive(&mut app)
        .select_project()
        .select_env("test")
        .load_specs();

    assert_that(&app)
        .loaded_spec(PROJECT_ENV_SPEC_ID)
        .loaded_ac(PROJECT_ENV_AC_ID)
        .under_spec(PROJECT_ENV_SPEC_ID)
        .missing_spec("dev-only-spec")
        .status_contains("Loaded test specs");

    Ok(())
}
