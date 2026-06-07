//! Dispatches `coherence-core-db db <subcommand>`.
//! Subcommands: truncate, export-jsonl, import-jsonl, list-databases

pub fn run(args: &[String]) -> i32 {
    let sub = args.first().map_or("", String::as_str);
    let tail = &args[1..];
    match sub {
        "truncate" => super::db_truncate::run(tail),
        "export-jsonl" => super::db_export_jsonl::run(tail),
        "import-jsonl" => super::db_import_jsonl::run(tail),
        "list-databases" => super::db_list_databases::run(),
        other => {
            eprintln!("db: unknown subcommand: {other} (expected truncate, export-jsonl, import-jsonl, list-databases)");
            1
        }
    }
}