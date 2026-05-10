use crate::db::{self, ConnectionConfig};
use crate::migrations;

pub fn run() -> i32 {
    if let Err(err) = db::preflight_user_scoped_migrate_binding() {
        eprintln!("migrate: failed");
        eprintln!("{err}");
        return 1;
    }
    let config = match ConnectionConfig::from_env() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("migrate: failed");
            eprintln!("{err}");
            return 1;
        }
    };
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
