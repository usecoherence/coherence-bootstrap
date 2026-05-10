use crate::commands;

pub fn run(args: Vec<String>) -> i32 {
    let command = args.get(1).map(String::as_str).unwrap_or("help");
    match command {
        "help" | "--help" | "-h" => {
            print_help();
            0
        }
        "doctor" => commands::doctor::run(),
        "migrate" => commands::migrate::run(),
        "m0-smoke" => commands::m0_smoke::run(),
        "m1-spec-smoke" => commands::m1_spec_smoke::run(),
        "spec" => commands::spec_cmd::run(&args[2..]),
        "ac" => commands::ac_cmd::run(&args[2..]),
        "verify-ac" => commands::verify_ac_cmd::run(&args[2..]),
        "verify-spec" => commands::verify_spec_cmd::run(&args[2..]),
        "version" | "--version" | "-V" => {
            println!("coherence-core-db 0.1.0");
            0
        }
        other => {
            eprintln!("unknown command: {other}");
            eprintln!("run: coherence-core-db help");
            64
        }
    }
}

fn print_help() {
    println!(
        "coherence-core-db\n\n\
         Commands:\n\
           help       Show this help\n\
           doctor     Check runtime assumptions\n\
           migrate    Run migrations via migration library\n\
           m0-smoke       Run minimal Rust -> Dolt DB vertical slice\n\
           m1-spec-smoke  Run M1 spec store smoke (Spec / AC / SpecRelation)\n\
           spec           Manage spec records (add, list, show)\n\
           ac             Manage acceptance criteria (add, list)\n\
           verify-ac      Run verified_by linked test commands for one AC\n\
           verify-spec    Aggregate verify-ac across all ACs for one spec\n\
           version        Print version\n\n\
         Canonical repository database:\n\
           Curated reasoning state lives in this checkout's Dolt catalog. Workspace tests never write it:\n\
           they require COHERENCE_DB_PROFILE=test and a disposable Dolt target.\n\
           Mutating smoke (m0-smoke / m1-spec-smoke) uses the same rule — prefer `make smoke` from repo root.\n\
           Details: AGENTS.md (canonical DB policy / test-world lifecycle).\n\n\
         Repository workflow:\n\
           make tool help\n\
           make tool doctor\n\
           make tool context\n\
           make tool next\n\
           make tool plan\n\
           make tool run\n\
           make tool present-work\n\
           make tool feedback"
    );
}
