use coherence_core_db::project_manifest::{effective_dolt_catalog_name, CoherenceEnv};
use coherence_core_db_tui::action::AppAction;
use coherence_core_db_tui::app::{AppState, Screen};
use coherence_core_db_tui::effects::{self, Effect};
use coherence_core_db_tui::update::update;
use coherence_test_world::{DoltWorld, EnvGuard, Scaffold};

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

pub const PROJECT_ENV_SPEC_ID: &str = "spec-dogfood-project-env-selection-test-world-m0";
pub const PROJECT_ENV_AC_ID: &str = "ac-product-tui-browser-project-env";

pub fn run_project_env_selection_spec_ac_e2e() {
    let slug = "project_env_selection_dogfood";
    let project_hash = "envhash";
    let dev_db = effective_dolt_catalog_name(slug, Some(project_hash), CoherenceEnv::Dev).unwrap();
    let test_db =
        effective_dolt_catalog_name(slug, Some(project_hash), CoherenceEnv::Test).unwrap();
    let prod_db =
        effective_dolt_catalog_name(slug, Some(project_hash), CoherenceEnv::Prod).unwrap();

    let scaffold = Scaffold::new(slug).unwrap();
    scaffold
        .write_file(
            ".coherence/project.toml",
            &format!(
                r#"
version = 2
project_slug = "{slug}"
project_hash = "{project_hash}"
dolt_mode = "user-scoped"
"#,
            ),
        )
        .unwrap();
    scaffold.init_git().unwrap();

    let dolt = DoltWorld::init(&dev_db).unwrap();
    dolt.create_database(&test_db).unwrap();
    dolt.create_database(&prod_db).unwrap();
    for db in [&dev_db, &test_db, &prod_db] {
        dolt.run_sql_in(db, CORE_DB_SCHEMA).unwrap();
    }
    dolt.run_sql_in(
        &dev_db,
        "INSERT INTO specs (id, slug, title, level, status) VALUES ('dev-only-spec', 'dev-only-spec', 'Dev Only Spec', 'product', 'draft')",
    )
    .unwrap();
    dolt.run_sql_in(
        &test_db,
        &format!(
            "INSERT INTO specs (id, slug, title, level, status) VALUES ('{PROJECT_ENV_SPEC_ID}', '{PROJECT_ENV_SPEC_ID}', 'Project Env Selection', 'product', 'draft')"
        ),
    )
    .unwrap();
    dolt.run_sql_in(
        &test_db,
        &format!(
            "INSERT INTO acceptance_criteria (id, spec_id, slug, title, intent, review_mode, risk_level) VALUES ('{PROJECT_ENV_AC_ID}', '{PROJECT_ENV_SPEC_ID}', 'project-env-selection', 'Loads selected env spec and AC', 'Selecting the test env loads the test-tier spec graph', 'automated', 'medium')"
        ),
    )
    .unwrap();

    let socket_path = std::env::temp_dir().join(format!("dolt_{slug}.sock"));
    let server = dolt.start_server(&socket_path).unwrap();
    let guard = EnvGuard::save(&[
        "DOLT_SOCKET",
        "DOLT_DB",
        "COHERENCE_DB_PROFILE",
        "COHERENCE_ENV",
    ])
    .unwrap();
    guard.set_current_dir(&scaffold.path(".")).unwrap();
    std::env::set_var(
        "DOLT_SOCKET",
        server.socket_path().to_string_lossy().as_ref(),
    );
    std::env::remove_var("DOLT_DB");
    std::env::set_var("COHERENCE_DB_PROFILE", "test");
    std::env::set_var("COHERENCE_ENV", "dev");

    let mut app = AppState::new(vec![(scaffold.path("."), slug.to_string())]);
    update(&mut app, AppAction::Enter);
    assert_eq!(app.screen, Screen::EnvPicker);
    update(&mut app, AppAction::NavDown);
    assert_eq!(app.envs[app.selected_env], "test");
    let effects = update(&mut app, AppAction::Enter);
    assert!(effects.contains(&Effect::RefreshGraph));

    effects::execute_effects(&mut app, effects);

    assert!(
        app.graph.is_some(),
        "test env graph should load: {}",
        app.status
    );
    let graph = app.graph.as_ref().unwrap();
    assert!(graph
        .specs
        .iter()
        .any(|spec| spec.id == PROJECT_ENV_SPEC_ID));
    assert!(graph
        .acceptance_criteria
        .iter()
        .any(|ac| ac.id == PROJECT_ENV_AC_ID && ac.spec_id == PROJECT_ENV_SPEC_ID));
    assert!(!graph.specs.iter().any(|spec| spec.id == "dev-only-spec"));
    assert!(
        app.status.contains("Loaded test specs"),
        "status: {}",
        app.status
    );
}
