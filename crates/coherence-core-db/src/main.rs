mod cli;
mod commands;
mod db;
mod migrations;
mod models;

fn main() {
    let code = cli::run(std::env::args().collect());
    std::process::exit(code);
}
