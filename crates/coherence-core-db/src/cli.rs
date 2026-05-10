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
        "db-ping" => commands::db_ping::run(),
        "drop-isolated-test-db" => commands::drop_isolated_test_db::run(),
        "m0-smoke" => commands::m0_smoke::run(),
        "m1-spec-smoke" => commands::m1_spec_smoke::run(),
        "spec" => commands::spec_cmd::run(&args[2..]),
        "ac" => commands::ac_cmd::run(&args[2..]),
        "ac-tests" => commands::ac_tests_cmd::run(&args[2..]),
        "verify-ac" => commands::verify_ac_cmd::run(&args[2..]),
        "verify-spec" => commands::verify_spec_cmd::run(&args[2..]),
        "evidence-sample" => commands::evidence_sample_cmd::run(&args[2..]),
        "project" => commands::project_cmd::run(&args[2..]),
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
           db-ping    Verify MySQL-protocol readiness (socket first, then TCP)\n\
           drop-isolated-test-db  Drop a coherence_test_* DB on user-scoped server (ADR-0004)\n\
           m0-smoke       Run minimal Rust -> Dolt DB vertical slice\n\
           m1-spec-smoke  Run M1 spec store smoke (Spec / AC / SpecRelation)\n\
           spec           Manage spec records (add, list, show)\n\
           ac             Manage acceptance criteria (add, list, show); each AC has a per-spec slug (default from id)\n\
           ac-tests       AC test files: materialize-rust (create missing tests/ac/**/*.rs), check-rust (hard gate vs graph)\n\
           verify-ac      Run verified_by linked test commands for one AC\n\
           verify-spec    Aggregate verify-ac across all ACs for one spec\n\
           evidence-sample  Create a demo run under .coherence/runs/<run-id>/ (ADR-0005 skeleton)\n\
           project        Project manifest — run `project init` to freeze dolt_db_name in .coherence/project.toml once project_slug exists (formula in project init usage); operator flow in AGENTS.md (“Project identity and manifest lifecycle”).\n\
           version        Print version\n\n\
         Canonical repository database — curated catalog vs disposable tests:\n\
           • Curated catalog — long-lived spec/AC/codeintel rows; never written by tests/smoke without isolation.\n\
           • Isolated test world — COHERENCE_DB_PROFILE=test and a disposable Dolt target (DOLT_DB naming on user-scoped Dolt).\n\
           • Per-run evidence — under .coherence/runs/<run-id>/ per ADR-0005; not heavy blobs in canonical tables.\n\
           • Optional user-scoped server — COHERENCE_USE_USER_SCOPED_DOLT=1: one dolt sql-server, pick catalog with DOLT_DB.\n\
           Key env: DOLT_SOCKET, DOLT_DB, COHERENCE_DB_PROFILE, COHERENCE_PROJECT_SLUG, COHERENCE_KEEP_TEST_WORLD.\n\n\
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
