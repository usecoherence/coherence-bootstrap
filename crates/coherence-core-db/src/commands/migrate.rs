use crate::db::ConnectionConfig;
use crate::migrations;

pub fn run() -> i32 {
    let config = ConnectionConfig::from_env();
    match migrations::apply_all(&config) {
        Ok(applied) => {
            println!("migrate: success");
            println!("database: {}", config.database);
            println!("applied_migrations: {applied}");
            0
        }
        Err(err) => {
            eprintln!("migrate: failed");
            eprintln!("{err}");
            1
        }
    }
}
