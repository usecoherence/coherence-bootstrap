fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    std::process::exit(coherence_core_db_bootstrap::run(&args));
}
