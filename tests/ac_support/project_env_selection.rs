use coherence_core_db::project_manifest::{effective_dolt_catalog_name, CoherenceEnv};
use coherence_core_db_tui::action::AppAction;
use coherence_core_db_tui::app::{AppState, Screen};
use coherence_core_db_tui::effects::{self, Effect};
use coherence_core_db_tui::update::update;
use coherence_test_world::{DoltServer, DoltWorld, EnvGuard, Scaffold};

const SLUG: &str = "project_env_selection_dogfood";
const PROJECT_HASH: &str = "envhash";
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

pub struct ProjectEnvSelectionWorldBuilder {
    dev_specs: Vec<String>,
    include_project_env_ac: bool,
}

pub struct ProjectEnvSelectionWorld {
    scaffold: Scaffold,
    _dolt: DoltWorld,
    _server: DoltServer,
    _guard: EnvGuard,
}

impl ProjectEnvSelectionWorld {
    pub fn builder() -> ProjectEnvSelectionWorldBuilder {
        ProjectEnvSelectionWorldBuilder {
            dev_specs: Vec::new(),
            include_project_env_ac: false,
        }
    }

    pub fn open_tui(&self) -> Result<AppState, String> {
        Ok(AppState::new(vec![(
            self.scaffold.path("."),
            SLUG.to_string(),
        )]))
    }

    pub fn drive<'a>(&self, app: &'a mut AppState) -> TuiDriver<'a> {
        TuiDriver { app }
    }
}

impl ProjectEnvSelectionWorldBuilder {
    pub fn with_dev_spec(mut self, spec_id: &str) -> Self {
        self.dev_specs.push(spec_id.to_string());
        self
    }

    pub fn with_test_project_env_ac(mut self) -> Self {
        self.include_project_env_ac = true;
        self
    }

    pub fn build(self) -> Result<ProjectEnvSelectionWorld, String> {
        let dbs = CatalogNames::new()?;
        let scaffold = create_project_scaffold()?;
        let dolt = create_seeded_dolt(&dbs, &self.dev_specs, self.include_project_env_ac)?;
        let server = dolt.start_server(&std::env::temp_dir().join(format!("dolt_{SLUG}.sock")))?;
        let guard = configure_process_env(&scaffold, &server)?;

        Ok(ProjectEnvSelectionWorld {
            scaffold,
            _dolt: dolt,
            _server: server,
            _guard: guard,
        })
    }
}

pub struct TuiDriver<'a> {
    app: &'a mut AppState,
}

impl TuiDriver<'_> {
    pub fn select_project(self) -> Self {
        assert_eq!(self.app.screen, Screen::ProjectPicker);
        update(self.app, AppAction::Enter);
        assert_eq!(self.app.screen, Screen::EnvPicker);
        self
    }

    pub fn select_env(self, env_name: &str) -> Self {
        assert_eq!(self.app.screen, Screen::EnvPicker);
        let target = self
            .app
            .envs
            .iter()
            .position(|env| env == env_name)
            .unwrap_or_else(|| panic!("env {env_name:?} not found in {:?}", self.app.envs));

        while self.app.selected_env < target {
            update(self.app, AppAction::NavDown);
        }
        while self.app.selected_env > target {
            update(self.app, AppAction::NavUp);
        }
        assert_eq!(self.app.envs[self.app.selected_env], env_name);
        self
    }

    pub fn load_specs(self) {
        let effects = update(self.app, AppAction::Enter);
        assert!(effects.contains(&Effect::RefreshGraph));
        effects::execute_effects(self.app, effects);
    }
}

pub fn assert_that(app: &AppState) -> AppAssertions<'_> {
    AppAssertions { app, last_ac: None }
}

pub struct AppAssertions<'a> {
    app: &'a AppState,
    last_ac: Option<&'a str>,
}

impl<'a> AppAssertions<'a> {
    pub fn loaded_spec(self, spec_id: &str) -> Self {
        let graph = self.loaded_graph();
        assert!(
            graph.specs.iter().any(|spec| spec.id == spec_id),
            "expected loaded spec {spec_id}, loaded specs: {:?}",
            graph
                .specs
                .iter()
                .map(|spec| spec.id.as_str())
                .collect::<Vec<_>>()
        );
        self
    }

    pub fn loaded_ac(mut self, ac_id: &'a str) -> Self {
        let graph = self.loaded_graph();
        assert!(
            graph.acceptance_criteria.iter().any(|ac| ac.id == ac_id),
            "expected loaded AC {ac_id}, loaded ACs: {:?}",
            graph
                .acceptance_criteria
                .iter()
                .map(|ac| ac.id.as_str())
                .collect::<Vec<_>>()
        );
        self.last_ac = Some(ac_id);
        self
    }

    pub fn under_spec(self, spec_id: &str) -> Self {
        let Some(ac_id) = self.last_ac else {
            panic!("under_spec() must follow loaded_ac(<id>)");
        };
        let graph = self.loaded_graph();
        let Some(ac) = graph.acceptance_criteria.iter().find(|ac| ac.id == ac_id) else {
            panic!("loaded_ac() already asserted AC {ac_id} exists");
        };
        assert_eq!(
            ac.spec_id, spec_id,
            "expected AC {ac_id} under spec {spec_id}"
        );
        self
    }

    pub fn missing_spec(self, spec_id: &str) -> Self {
        let graph = self.loaded_graph();
        assert!(
            !graph.specs.iter().any(|spec| spec.id == spec_id),
            "expected missing spec {spec_id}, loaded specs: {:?}",
            graph
                .specs
                .iter()
                .map(|spec| spec.id.as_str())
                .collect::<Vec<_>>()
        );
        self
    }

    pub fn status_contains(self, expected: &str) -> Self {
        assert!(
            self.app.status.contains(expected),
            "expected status to contain {expected:?}, got {:?}",
            self.app.status
        );
        self
    }

    fn loaded_graph(&self) -> &coherence_core_db::models::SpecGraph {
        self.app
            .graph
            .as_ref()
            .unwrap_or_else(|| panic!("expected graph to load, status: {}", self.app.status))
    }
}

struct CatalogNames {
    dev: String,
    test: String,
    prod: String,
}

impl CatalogNames {
    fn new() -> Result<Self, String> {
        Ok(Self {
            dev: effective_dolt_catalog_name(SLUG, Some(PROJECT_HASH), CoherenceEnv::Dev)?,
            test: effective_dolt_catalog_name(SLUG, Some(PROJECT_HASH), CoherenceEnv::Test)?,
            prod: effective_dolt_catalog_name(SLUG, Some(PROJECT_HASH), CoherenceEnv::Prod)?,
        })
    }
}

fn create_project_scaffold() -> Result<Scaffold, String> {
    let scaffold = Scaffold::new(SLUG)?;
    scaffold.write_file(
        ".coherence/project.toml",
        &format!(
            r#"
version = 2
project_slug = "{SLUG}"
project_hash = "{PROJECT_HASH}"
dolt_mode = "user-scoped"
"#,
        ),
    )?;
    scaffold.init_git()?;
    Ok(scaffold)
}

fn create_seeded_dolt(
    dbs: &CatalogNames,
    dev_specs: &[String],
    include_project_env_ac: bool,
) -> Result<DoltWorld, String> {
    let dolt = DoltWorld::init(&dbs.dev)?;
    dolt.create_database(&dbs.test)?;
    dolt.create_database(&dbs.prod)?;
    for db in [&dbs.dev, &dbs.test, &dbs.prod] {
        dolt.run_sql_in(db, CORE_DB_SCHEMA)?;
    }
    for spec_id in dev_specs {
        seed_spec(&dolt, &dbs.dev, spec_id, "Dev Only Spec")?;
    }
    if include_project_env_ac {
        seed_spec(
            &dolt,
            &dbs.test,
            PROJECT_ENV_SPEC_ID,
            "Project Env Selection",
        )?;
        seed_ac(&dolt, &dbs.test, PROJECT_ENV_AC_ID, PROJECT_ENV_SPEC_ID)?;
    }
    Ok(dolt)
}

fn seed_spec(dolt: &DoltWorld, db: &str, spec_id: &str, title: &str) -> Result<(), String> {
    dolt.run_sql_in(
        db,
        &format!(
            "INSERT INTO specs (id, slug, title, level, status) VALUES ('{spec_id}', '{spec_id}', '{title}', 'product', 'draft')"
        ),
    )?;
    Ok(())
}

fn seed_ac(dolt: &DoltWorld, db: &str, ac_id: &str, spec_id: &str) -> Result<(), String> {
    dolt.run_sql_in(
        db,
        &format!(
            "INSERT INTO acceptance_criteria (id, spec_id, slug, title, intent, review_mode, risk_level) VALUES ('{ac_id}', '{spec_id}', 'project-env-selection', 'Loads selected env spec and AC', 'Selecting the test env loads the test-tier spec graph', 'automated', 'medium')"
        ),
    )?;
    Ok(())
}

fn configure_process_env(scaffold: &Scaffold, server: &DoltServer) -> Result<EnvGuard, String> {
    let guard = EnvGuard::save(&[
        "DOLT_SOCKET",
        "DOLT_DB",
        "COHERENCE_DB_PROFILE",
        "COHERENCE_ENV",
    ])?;
    guard.set_current_dir(&scaffold.path("."))?;
    std::env::set_var(
        "DOLT_SOCKET",
        server.socket_path().to_string_lossy().as_ref(),
    );
    std::env::remove_var("DOLT_DB");
    std::env::set_var("COHERENCE_DB_PROFILE", "test");
    std::env::set_var("COHERENCE_ENV", "dev");
    Ok(guard)
}
