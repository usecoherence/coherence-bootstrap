mod cli;
mod commands;
mod db;
mod models;

fn main() {
    let code = cli::run(std::env::args().collect());
    std::process::exit(code);
}
