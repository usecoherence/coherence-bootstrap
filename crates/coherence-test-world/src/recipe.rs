use std::collections::HashMap;
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
    env: HashMap<String, String>,
}

impl E2eEnvironment {
    pub fn env(&self) -> HashMap<String, String> {
        let mut env = self.env.clone();
        env.insert(
            "DOLT_SOCKET".to_string(),
            self.server.socket_path().to_string_lossy().to_string(),
        );
        env.insert("DOLT_DB".to_string(), self.db_name.clone());
        env
    }
}

/// Declarative builder that replaces the handwritten `setup_e2e_env()` boilerplate.
///
/// ```ignore
/// let env = E2eRecipe::default()
///     .migrate_sql(include_str!("schema.sql"))
///     .seed_sql("INSERT INTO examples (id) VALUES ('one')")
///     .build()?;
/// ```
pub struct E2eRecipe {
    slug: String,
    migration_sql: Vec<String>,
    seed_sql: Vec<String>,
    env: HashMap<String, String>,
}

impl Default for E2eRecipe {
    fn default() -> Self {
        let n = RECIPE_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            slug: format!("e2e_project_{n}"),
            migration_sql: Vec::new(),
            seed_sql: Vec::new(),
            env: HashMap::new(),
        }
    }
}

impl E2eRecipe {
    pub fn named(prefix: &str) -> Self {
        let n = RECIPE_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            slug: format!("{prefix}_{n}"),
            migration_sql: Vec::new(),
            seed_sql: Vec::new(),
            env: HashMap::new(),
        }
    }

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

    pub fn migrate_sql(mut self, sql: &str) -> Self {
        self.migration_sql.push(sql.to_string());
        self
    }

    pub fn seed_sql(mut self, sql: &str) -> Self {
        self.seed_sql.push(sql.to_string());
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }

    /// Build the environment: scaffold + `dolt_world` + server + seed data.
    ///
    /// # Errors
    ///
    /// Returns an error if scaffold creation, Dolt init, SQL migration/seeding, or
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

        for sql in &self.migration_sql {
            dolt_world.run_sql(sql)?;
        }

        for sql in &self.seed_sql {
            dolt_world.run_sql(sql)?;
        }

        let server = dolt_world.start_server(&socket_path)?;

        Ok(E2eEnvironment {
            scaffold,
            dolt_world,
            server,
            slug: slug.clone(),
            db_name,
            env: self.env.clone(),
        })
    }
}
