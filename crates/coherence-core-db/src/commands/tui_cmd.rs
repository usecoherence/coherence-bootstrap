use std::process::Command;

pub fn run(args: &[String]) -> i32 {
    if args.iter().any(|arg| matches!(arg.as_str(), "-h" | "--help")) {
        print_help();
        return 0;
    }

    match Command::new("coherence-core-db-tui").args(args).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!("failed to launch coherence-core-db-tui: {err}");
            eprintln!("install/build the TUI binary, then retry: coherence-core-db tui");
            127
        }
    }
}

fn print_help() {
    println!(
        "coherence-core-db tui\n\n\
         Launch the coherence-core-db-tui binary through the main CLI entrypoint.\n\n\
         Usage:\n\
           coherence-core-db tui\n\n\
         Requires `coherence-core-db-tui` to be available on PATH."
    );
}
