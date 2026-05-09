mod cli;
mod commands;

fn main() {
    let code = cli::run(std::env::args().collect());
    std::process::exit(code);
}
