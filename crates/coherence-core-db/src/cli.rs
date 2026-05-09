use crate::commands;

pub fn run(args: Vec<String>) -> i32 {
    let command = args.get(1).map(String::as_str).unwrap_or("help");
    match command {
        "help" | "--help" | "-h" => {
            print_help();
            0
        }
        "doctor" => commands::doctor::run(),
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
           version    Print version\n\n\
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
