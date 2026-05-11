pub fn run(args: &[String]) -> i32 {
    let sub = match args.first().map(|s| s.as_str()) {
        Some(s) => s,
        None => {
            eprintln!("usage: coherence-core-db project <subcommand>");
            eprintln!(
                "project catalog-preflight  verify git root + `.coherence/project.toml` slug/hash binding before db-ping/dolt-start (shared with doctor)"
            );
            eprintln!(
                "project init binds project_hash (and derived legacy dolt_db_name) in .coherence/project.toml after project_slug is set — see AGENTS.md (\"Project identity and manifest lifecycle\")."
            );
            eprintln!(
                "project reset  idempotent repair: keeps project_slug, runs init bind-if-needed, then migrate (Dolt must be up)"
            );
            eprintln!("run: coherence-core-db help");
            return 64;
        }
    };
    let tail = &args[1..];
    match sub {
        "catalog-preflight" => {
            match crate::db::manifest_catalog_preflight_for_connect("catalog-preflight") {
                Ok(()) => {
                    println!("catalog-preflight: ok");
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            }
        }
        "init" => super::project_init_cmd::run(tail),
        "reset" => {
            if !tail.is_empty() {
                eprintln!("project reset: unexpected arguments (this command takes no options)");
                eprintln!("run: coherence-core-db project reset");
                return 64;
            }
            super::project_reset_cmd::run()
        }
        other => {
            eprintln!(
                "unknown project subcommand: {other} (expected: catalog-preflight | init | reset)"
            );
            eprintln!(
                "see AGENTS.md (Project identity and manifest lifecycle) for manifest setup."
            );
            eprintln!("run: coherence-core-db help");
            64
        }
    }
}
