use crate::db::{self, ConnectionConfig};

pub fn run() -> i32 {
    let config = ConnectionConfig::from_env();
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
