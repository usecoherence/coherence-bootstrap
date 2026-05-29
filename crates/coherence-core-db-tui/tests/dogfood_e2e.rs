// Smoke tests for coherence-test-world + TUI AppState.
// These verify basic scaffolding and navigation, but do NOT drive a real
// Dolt sql-server + DoltSpecRepository pipeline.  That requires COREDB-9d0.7.
#![allow(clippy::pedantic)]

use coherence_core_db_tui::action::AppAction;
use coherence_core_db_tui::app::{AppState, Screen};
use coherence_core_db_tui::effects::Effect;
use coherence_core_db_tui::update::update;
use coherence_test_world::{AcTest, DoltWorld, Evidence, Scaffold, VerificationResult, World};

#[test]
fn scaffold_creates_coherence_project_dir() {
    let s = Scaffold::new("dogfood").unwrap();
    s.write_file(".coherence/project.toml", r#"
project_slug = "test-project"
project_hash = "test123"
dolt_db_name = "test_project_test123_dev"
"#)
    .unwrap();
    s.init_git().unwrap();
    assert!(s.exists(".coherence/project.toml"));
    assert!(s.exists(".git"));
}

#[test]
fn dolt_world_seeds_and_queries_specs() {
    let dw = DoltWorld::init("dogfood_specs").unwrap();
    dw.run_sql(
        "INSERT INTO specs (id, slug, title, level, status) VALUES ('s1', 'spec-1', 'Test Spec', 'product', 'draft')",
    )
    .unwrap();
    let result = dw.run_sql("SELECT COUNT(*) AS c FROM specs").unwrap();
    assert!(result.contains("1") || result.contains("| 1 |"));
}

#[test]
fn acet_evaluates_pass_fail_skip() {
    let passed = Evidence {
        stdout: "ok".into(),
        stderr: String::new(),
        exit_code: Some(0),
        file_snapshots: std::collections::HashMap::new(),
    };
    assert_eq!(AcTest::pass_with(passed), VerificationResult::Passed);

    let failed = Evidence {
        stdout: String::new(),
        stderr: "error".into(),
        exit_code: Some(1),
        file_snapshots: std::collections::HashMap::new(),
    };
    assert!(matches!(AcTest::pass_with(failed), VerificationResult::Failed(_)));

    let skipped = Evidence::new();
    assert_eq!(
        AcTest::pass_with(skipped),
        VerificationResult::Skipped("No command executed".into())
    );
}

#[test]
fn filesystem_world_runs_commands() {
    let s = Scaffold::new("dogfood").unwrap();
    s.write_file("hello.txt", "world").unwrap();
    let world = World::Filesystem(s);
    let evidence = world.run_command("cat hello.txt").unwrap();
    assert_eq!(evidence.stdout.trim(), "world");
    assert_eq!(evidence.exit_code, Some(0));
}

#[test]
fn appstate_navigates_project_to_specs() {
    let s = Scaffold::new("dogfood").unwrap();
    s.write_file(".coherence/project.toml", "project_slug = \"test-project\"\n")
        .unwrap();
    let projects = vec![(s.path("."), "test-project".into())];
    let mut app = AppState::new(projects);

    assert_eq!(app.screen, Screen::ProjectPicker);

    let effects = update(&mut app, AppAction::Enter);
    assert_eq!(app.screen, Screen::EnvPicker);
    assert!(effects.is_empty());

    let effects = update(&mut app, AppAction::Enter);
    assert_eq!(app.screen, Screen::Specs);
    assert!(effects.contains(&Effect::RefreshGraph));
}

#[test]
fn appstate_key_edit_mode_creates_draft() {
    let s = Scaffold::new("dogfood").unwrap();
    s.write_file(".coherence/project.toml", "project_slug = \"test-project\"\n")
        .unwrap();
    let projects = vec![(s.path("."), "test-project".into())];
    let mut app = AppState::new(projects);

    update(&mut app, AppAction::Enter);
    update(&mut app, AppAction::Enter);
    assert_eq!(app.screen, Screen::Specs);

    app.detail_spec_id = Some("dummy-id".into());
    update(&mut app, AppAction::EnterEditMode);
    assert!(app.edit_mode);
}
