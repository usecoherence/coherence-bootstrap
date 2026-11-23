//! Old top-level commands. They stay reachable for one release.

pub fn run(args: &[String]) -> i32 {
    let command = args.get(1).map_or("help", String::as_str);
    match command {
        "code-quality" => match args.get(2).map_or("help", String::as_str) {
            "codescene-xray" => coherence_code_quality::codescene_xray::run(&args[3..]),
            "help" | "--help" | "-h" => {
                println!("{}", code_quality_help());
                0
            }
            _ => {
                eprintln!(
                    "usage: coherence code-quality <subcommand>\n\n{}",
                    code_quality_help()
                );
                1
            }
        },
        "version" => {
            println!("coherence {}", env!("CARGO_PKG_VERSION"));
            0
        }
        _ => coherence_core_db::cli::run(args),
    }
}

fn code_quality_help() -> &'static str {
    "code-quality subcommands:\n\
         codescene-xray  File-level X-Ray report via CodeScene\n\
         help            Show this help"
}
