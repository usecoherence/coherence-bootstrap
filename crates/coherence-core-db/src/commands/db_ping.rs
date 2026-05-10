use crate::db::{self, ConnectionConfig};

pub fn run() -> i32 {
    if let Err(err) = db::manifest_catalog_preflight_for_connect("db-ping") {
        eprintln!("db-ping: failed");
        eprintln!("{err}");
        return 1;
    }
    let config = match ConnectionConfig::from_env() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("db-ping: failed");
            eprintln!("{err}");
            return 1;
        }
    };
    match db::ping_server(&config) {
        Ok(mode) => {
            println!("db-ping: ok ({mode})");
            0
        }
        Err(err) => {
            eprintln!("db-ping: failed");
            eprintln!("{err}");
            1
        }
    }
}
