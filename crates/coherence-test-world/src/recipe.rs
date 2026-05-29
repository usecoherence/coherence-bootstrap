use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::dolt_world::{DoltServer, DoltWorld};
use crate::scaffold::Scaffold;

static RECIPE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Result of building an [`E2eRecipe`]: a fully wired Dolt-backed environment.
pub struct E2eEnvironment {
    pub scaffold: Scaffold,
    pub dolt_world: DoltWorld,
    pub server: DoltServer,
    pub slug: String,
    pub db_name: String,
}

/// Declarative builder that replaces the handwritten `setup_e2e_env()` boilerplate.
///
/// ```ignore
/// let env = E2eRecipe::default()
///     .spec("e2e-spec-1", "E2E Test Spec", "product", "draft")
///     .build()?;
/// ```
pub struct E2eRecipe {
    slug: String,
    specs: Vec<(String, String, String, String)>,
}

impl Default for E2eRecipe {
    fn default() -> Self {
        let n = RECIPE_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            slug: format!("e2e_project_{n}"),
            specs: Vec::new(),
        }
    }
}

impl E2eRecipe {
    pub fn with_slug(mut self, slug: &str) -> Self {
        self.slug = slug.to_string();
        self
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }

    pub fn db_name(&self) -> String {
        format!("{}_dev", self.slug)
    }

    pub fn default_socket_path(&self) -> PathBuf {
        std::env::temp_dir().join(format!("dolt_{}.sock", self.slug))
    }

    /// Register a spec to seed into the Dolt database.
    pub fn spec(mut self, id: &str, title: &str, level: &str, status: &str) -> Self {
        self.specs.push((id.to_string(), title.to_string(), level.to_string(), status.to_string()));
        self
    }

    /// Build the environment: scaffold + `dolt_world` + server + seed data.
    ///
    /// # Errors
    ///
    /// Returns an error if scaffold creation, Dolt init, SQL seeding, or
    /// server startup fails.
    pub fn build(&self) -> Result<E2eEnvironment, String> {
        let slug = &self.slug;
        let db_name = self.db_name();
        let socket_path = self.default_socket_path();

        let scaffold = Scaffold::new(slug)?;
        scaffold.write_file(
            ".coherence/project.toml",
            &format!(
                r#"
project_slug = "{slug}"
dolt_db_name = "{db_name}"
dolt_mode = "user-scoped"
"#,
            ),
        )?;
        scaffold.init_git()?;

        let dolt_world = DoltWorld::init(&db_name)?;

        for (id, title, level, status) in &self.specs {
            dolt_world.run_sql(&format!(
                "INSERT INTO specs (id, slug, title, level, status) \
                 VALUES ('{id}', '{id}', '{title}', '{level}', '{status}')",
            ))?;
        }

        let server = dolt_world.start_server(&socket_path)?;

        Ok(E2eEnvironment {
            scaffold,
            dolt_world,
            server,
            slug: slug.clone(),
            db_name,
        })
    }
}
