//! Applies all embedded SQL migrations to the repo-local DB.
//!
//! **Logical modules**: `spec_module` (`sql/modules/spec/migrations`) and `codeintel_module`
//! (`sql/modules/codeintel/migrations`) share one physical database; each has its own owner for
//! table design and evolves through its migration chain. coherence-core-db runs both via Refinery,
//! including a distinct history table for codeintel (`CODEINTEL_MIGRATION_TABLE`) so version IDs
//! do not collide. See `AGENTS.md` → “M1 module ownership”.
use refinery::config::ConfigDbType;

use crate::db::ConnectionConfig;

mod spec_module {
    use refinery::embed_migrations;
    embed_migrations!("sql/modules/spec/migrations");
}

mod codeintel_module {
    use refinery::embed_migrations;
    embed_migrations!("sql/modules/codeintel/migrations");
}

/// Separate refinery history table for the codeintel module so version numbers (e.g. V1) do not
/// collide with the spec module's migration set on the same database.
const CODEINTEL_MIGRATION_TABLE: &str = "refinery_schema_history_codeintel";

pub fn apply_all(config: &ConnectionConfig) -> Result<usize, String> {
    crate::db::ensure_project_database(config)?;

    // SQL migrations are embedded at compile time via refinery.
    let mut refinery_config = refinery::config::Config::new(ConfigDbType::Mysql)
        .set_db_name(&config.database)
        .set_db_user(&config.user)
        .set_db_host(&config.host)
        .set_db_port(&config.port.to_string());
    if let Some(password) = &config.password {
        refinery_config = refinery_config.set_db_pass(password);
    }

    let report_spec = spec_module::migrations::runner()
        .set_grouped(false)
        .run(&mut refinery_config)
        .map_err(|err| format!("failed to apply spec refinery migrations: {err}"))?;

    let mut codeintel_runner = codeintel_module::migrations::runner().set_grouped(false);
    codeintel_runner.set_migration_table_name(CODEINTEL_MIGRATION_TABLE);
    let report_codeintel = codeintel_runner
        .run(&mut refinery_config)
        .map_err(|err| format!("failed to apply codeintel refinery migrations: {err}"))?;

    Ok(report_spec.applied_migrations().len() + report_codeintel.applied_migrations().len())
}
