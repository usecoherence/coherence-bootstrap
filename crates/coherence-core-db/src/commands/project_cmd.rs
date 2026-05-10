pub fn run(args: &[String]) -> i32 {
    let sub = match args.first().map(|s| s.as_str()) {
        Some(s) => s,
        None => {
            eprintln!("usage: coherence-core-db project init [OPTIONS]");
            eprintln!(
                "project catalog-preflight  verify git root + `.coherence/project.toml` slug/hash binding before db-ping/dolt-start (shared with doctor)"
            );
            eprintln!(
                "project init binds project_hash (and derived legacy dolt_db_name) in .coherence/project.toml after project_slug is set — see AGENTS.md (\"Project identity and manifest lifecycle\")."
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
        other => {
            eprintln!("unknown project subcommand: {other} (expected: catalog-preflight | init)");
            eprintln!(
                "see AGENTS.md (Project identity and manifest lifecycle) for manifest setup."
            );
            eprintln!("run: coherence-core-db help");
            64
        }
    }
}
