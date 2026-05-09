use refinery::config::ConfigDbType;

use crate::db::ConnectionConfig;

mod spec_module {
    use refinery::embed_migrations;
    embed_migrations!("sql/modules/spec/migrations");
}

pub fn apply_all(config: &ConnectionConfig) -> Result<usize, String> {
    // SQL migrations are embedded at compile time via refinery.
    let mut refinery_config = refinery::config::Config::new(ConfigDbType::Mysql)
        .set_db_name(&config.database)
        .set_db_user(&config.user)
        .set_db_host(&config.host)
        .set_db_port(&config.port.to_string());
    if let Some(password) = &config.password {
        refinery_config = refinery_config.set_db_pass(password);
    }

    let report = spec_module::migrations::runner()
        .set_grouped(false)
        .run(&mut refinery_config)
        .map_err(|err| format!("failed to apply refinery migrations: {err}"))?;
    Ok(report.applied_migrations().len())
}
