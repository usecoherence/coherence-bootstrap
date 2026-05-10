pub fn run(args: &[String]) -> i32 {
    let sub = match args.first().map(|s| s.as_str()) {
        Some(s) => s,
        None => {
            eprintln!("usage: coherence-core-db project init [OPTIONS]");
            eprintln!("run: coherence-core-db help");
            return 64;
        }
    };
    let tail = &args[1..];
    match sub {
        "init" => super::project_init_cmd::run(tail),
        other => {
            eprintln!("unknown project subcommand: {other} (expected: init)");
            eprintln!("run: coherence-core-db help");
            64
        }
    }
}
