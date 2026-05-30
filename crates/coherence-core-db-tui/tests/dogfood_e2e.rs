// Smoke + e2e tests for coherence-test-world + TUI AppState.
#![allow(clippy::pedantic, clippy::unwrap_used)]

use coherence_core_db_tui::action::AppAction;
use coherence_core_db_tui::app::{AppState, Screen};
use coherence_core_db_tui::effects::Effect;
use coherence_core_db_tui::update::update;
use coherence_core_db_tui::effects;
use coherence_test_world::{
    AcTest, DoltWorld, E2eRecipe, EnvGuard, Evidence, Scaffold, VerificationResult, World,
};

const CORE_DB_SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS specs (
    id VARCHAR(255) PRIMARY KEY,
    slug VARCHAR(255) NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    level VARCHAR(50) NOT NULL DEFAULT 'module',
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    created_at VARCHAR(50) NOT NULL DEFAULT '',
    updated_at VARCHAR(50) NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS acceptance_criteria (
    id VARCHAR(255) PRIMARY KEY,
    spec_id VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL,
    title VARCHAR(255) NOT NULL,
    intent TEXT NOT NULL DEFAULT '',
    review_mode VARCHAR(50) NOT NULL DEFAULT 'manual',
    risk_level VARCHAR(50) NOT NULL DEFAULT 'medium',
    created_at VARCHAR(50) NOT NULL DEFAULT '',
    updated_at VARCHAR(50) NOT NULL DEFAULT '',
    FOREIGN KEY (spec_id) REFERENCES specs(id)
);

CREATE TABLE IF NOT EXISTS acceptance_criterion_concerns (
    id VARCHAR(255) PRIMARY KEY,
    acceptance_criterion_id VARCHAR(255) NOT NULL,
    concern TEXT NOT NULL,
    created_at VARCHAR(50) NOT NULL DEFAULT '',
    updated_at VARCHAR(50) NOT NULL DEFAULT '',
    FOREIGN KEY (acceptance_criterion_id) REFERENCES acceptance_criteria(id)
);

CREATE TABLE IF NOT EXISTS spec_relations (
    id VARCHAR(255) PRIMARY KEY,
    source_spec_id VARCHAR(255) NOT NULL,
    target_spec_id VARCHAR(255) NOT NULL,
    relation_kind VARCHAR(50) NOT NULL DEFAULT 'depends_on',
    note TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (source_spec_id) REFERENCES specs(id),
    FOREIGN KEY (target_spec_id) REFERENCES specs(id)
);
";

const E2E_SPEC_SEED: &str =
    "INSERT INTO specs (id, slug, title, level, status) VALUES ('e2e-spec-1', 'e2e-spec-1', 'E2E Test Spec', 'product', 'draft')";

// ---- Primitive smoke tests (no Dolt server needed) ----

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
    dw.run_sql(CORE_DB_SCHEMA)
        .unwrap();
    dw.run_sql("INSERT INTO specs (id, slug, title, level, status) VALUES ('s1', 'spec-1', 'Test Spec', 'product', 'draft')")
    .unwrap();
    let count = dw.run_sql("SELECT COUNT(*) AS c FROM specs").unwrap();
    assert!(count.contains("1") || count.contains("| 1 |"));
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
    let evidence = World::Filesystem(s).run_command("cat hello.txt").unwrap();
    assert_eq!(evidence.stdout.trim(), "world");
    assert_eq!(evidence.exit_code, Some(0));
}

// ---- AppState navigation smoke (no Dolt server) ----

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
    app.detail_spec_id = Some("dummy-id".into());
    update(&mut app, AppAction::EnterEditMode);
    assert!(app.edit_mode);
}

// ---- Real e2e: Dolt sql-server + DoltSpecRepository + execute_effects ----

#[test]
fn real_e2e_dolt_server_loads_spec_graph() {
    let env = E2eRecipe::default()
        .migrate_sql(CORE_DB_SCHEMA)
        .seed_sql(E2E_SPEC_SEED)
        .build()
        .unwrap();

    let guard = EnvGuard::save(&["DOLT_SOCKET", "DOLT_DB", "COHERENCE_DB_PROFILE", "COHERENCE_ENV"])
        .unwrap();

    // cd into scaffold dir so DoltSpecRepository reads its project.toml
    guard.set_current_dir(&env.scaffold.path(".")).unwrap();

    // Set env for DoltSpecRepository to connect to our test server
    std::env::set_var("DOLT_SOCKET", env.server.socket_path().to_string_lossy().as_ref());
    std::env::set_var("DOLT_DB", &env.db_name);
    std::env::set_var("COHERENCE_DB_PROFILE", "test");
    std::env::set_var("COHERENCE_ENV", "dev");

    let projects = vec![(env.scaffold.path("."), env.slug.clone())];
    let mut app = AppState::new(projects);

    // Navigate to Specs screen → triggers RefreshGraph effect
    update(&mut app, AppAction::Enter);
    let effects = update(&mut app, AppAction::Enter);
    assert!(effects.contains(&Effect::RefreshGraph));

    // Execute effects — this calls DoltSpecRepository::new + load_spec_graph
    effects::execute_effects(&mut app, effects);

    // Assert the spec graph loaded with our seeded spec
    assert!(
        app.graph.is_some(),
        "Expected SpecGraph to be loaded from Dolt sql-server"
    );
    let graph = app.graph.as_ref().unwrap();
    let spec_ids: Vec<&str> = graph.specs.iter().map(|s| s.id.as_str()).collect();
    assert!(
        spec_ids.contains(&"e2e-spec-1"),
        "Expected e2e-spec-1 in graph, got: {spec_ids:?}"
    );

    // Assert tree was built
    assert!(!app.tree_items.is_empty(), "Expected tree items after graph load");

    // Assert status indicates success
    assert!(
        !app.status.contains("failed") && !app.status.contains("error"),
        "Status should indicate success: {}",
        app.status
    );
    // EnvGuard restores env vars and current_dir on Drop
}
