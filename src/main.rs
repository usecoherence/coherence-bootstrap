fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let code = run(&args);
    std::process::exit(code);
}

fn run(args: &[String]) -> i32 {
    let command = args.get(1).map_or("help", String::as_str);
    match command {
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

fn print_help() {
    println!(
        "coherence-bootstrap\n\n\
         Facade entrypoint for the bootstrap coherence stack.\n\n\
         Commands:\n\
           tui            Launch the TUI in-process\n\
           spec           Delegate to coherence-core-db spec\n\
           ac             Delegate to coherence-core-db ac\n\
           ac-tests       Delegate to coherence-core-db ac-tests\n\
           verify-ac      Delegate to coherence-core-db verify-ac\n\
           verify-spec    Delegate to coherence-core-db verify-spec\n\
           project        Delegate to coherence-core-db project\n\
           doctor         Delegate to coherence-core-db doctor\n\
           migrate        Delegate to coherence-core-db migrate\n\
           help           Show this help\n\n\
         Most non-TUI commands use the same syntax as coherence-core-db."
    );
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
}
