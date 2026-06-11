fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let code = run(&args);
    std::process::exit(code);
}

fn run(args: &[String]) -> i32 {
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
                    "usage: coherence-bootstrap code-quality <subcommand>\n\n{}",
                    code_quality_help()
                );
                1
            }
        },
        "help" | "--help" | "-h" => {
            print_help();
            0
        }
        "tui" => {
            if args
                .iter()
                .skip(2)
                .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
            {
                print_tui_help();
                return 0;
            }

            match coherence_core_db_tui::run_terminal() {
                Ok(()) => 0,
                Err(err) => {
                    eprintln!("coherence-bootstrap tui: {err}");
                    1
                }
            }
        }
        "version" | "--version" | "-V" => {
            println!("coherence-bootstrap {}", env!("CARGO_PKG_VERSION"));
            0
        }
        _ => coherence_core_db::cli::run(args),
    }
}

fn print_tui_help() {
    println!(
        "coherence-bootstrap tui\n\n\
         Launch the bootstrap TUI in-process.\n\n\
         Usage:\n\
           coherence-bootstrap tui"
    );
}

fn code_quality_help() -> &'static str {
    "code-quality subcommands:\n\
         codescene-xray  File-level X-Ray report via CodeScene\n\
         help            Show this help"
}

fn print_help() {
    println!("{}", help_text());
}

fn help_text() -> &'static str {
    "coherence-bootstrap\n\n\
         Facade entrypoint for the bootstrap coherence stack.\n\n\
         Commands:\n\
            code-quality   Code quality analysis tools (codescene-xray)\n\
            tui            Launch the TUI in-process\n\
            spec           Delegate to coherence-core-db spec\n\
            ac             Delegate to coherence-core-db ac\n\
            ac-tests       Delegate to coherence-core-db ac-tests\n\
            db             DBA operations: truncate, export-jsonl, import-jsonl, list-databases\n\
            db-ping        Delegate to coherence-core-db db-ping\n\
            db-list-databases  Delegate to coherence-core-db db-list-databases\n\
            drop-isolated-test-db  Delegate to coherence-core-db drop-isolated-test-db\n\
            verify-ac      Delegate to coherence-core-db verify-ac\n\
            verify-spec    Delegate to coherence-core-db verify-spec\n\
            evidence-sample  Delegate to coherence-core-db evidence-sample\n\
            project        Delegate to coherence-core-db project\n\
             doctor         Delegate to coherence-core-db doctor\n\
             migrate        Delegate to coherence-core-db migrate\n\
            m0-smoke       Delegate to coherence-core-db m0-smoke\n\
            m1-spec-smoke  Delegate to coherence-core-db m1-spec-smoke\n\
            help           Show this help\n\n\
         Most non-TUI commands use the same syntax as coherence-core-db."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_exits_successfully() {
        let args = vec!["coherence-bootstrap".to_string(), "help".to_string()];
        assert_eq!(run(&args), 0);
    }

    #[test]
    fn core_commands_delegate() {
        let args = vec!["coherence-bootstrap".to_string(), "version".to_string()];
        assert_eq!(run(&args), 0);
    }

    #[test]
    fn tui_help_does_not_launch_terminal() {
        let args = vec![
            "coherence-bootstrap".to_string(),
            "tui".to_string(),
            "--help".to_string(),
        ];
        assert_eq!(run(&args), 0);
    }

    #[test]
    fn help_lists_code_quality() {
        let help = help_text();
        assert!(help.contains("code-quality"));
    }

    #[test]
    fn code_quality_help_works() {
        let args = vec![
            "coherence-bootstrap".to_string(),
            "code-quality".to_string(),
            "help".to_string(),
        ];
        assert_eq!(run(&args), 0);
    }

    #[test]
    fn code_quality_no_subcommand_shows_help() {
        let args = vec![
            "coherence-bootstrap".to_string(),
            "code-quality".to_string(),
        ];
        assert_eq!(run(&args), 0);
    }

    #[test]
    fn help_lists_db_import_export_operations() {
        let help = help_text();
        assert!(help.contains("db"));
        assert!(help.contains("truncate"));
        assert!(help.contains("export-jsonl"));
        assert!(help.contains("import-jsonl"));
    }
}
