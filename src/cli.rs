//! `clap` command surface for the `coherence` binary.
//!
//! This module only defines the CLI and maps each command to a handler. The
//! handlers still call the `coherence-core-db` command modules. Later PRs move
//! the logic into the new layout.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::legacy;

/// Legacy top-level commands. They stay reachable for one release.
const LEGACY_COMMANDS: &[&str] = &[
    "code-quality",
    "doctor",
    "migrate",
    "db-ping",
    "db-list-databases",
    "drop-isolated-test-db",
    "m0-smoke",
    "m1-spec-smoke",
    "spec",
    "ac",
    "ac-tests",
    "verify-ac",
    "verify-spec",
    "evidence-sample",
    "project",
    "version",
];

/// Legacy `db` subcommands. The new `db` surface uses different names.
const LEGACY_DB_SUBCOMMANDS: &[&str] =
    &["truncate", "export-jsonl", "import-jsonl", "list-databases"];

#[derive(Debug, Parser)]
#[command(
    name = "coherence",
    version,
    about = "Local-first specification database",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create or repair the project database
    Init(InitArgs),
    /// Database operations
    Db {
        #[command(subcommand)]
        command: DbCommand,
    },
    /// Open the terminal UI
    Tui,
    /// Verify specs and acceptance criteria
    Verify(VerifyArgs),
    /// Create, edit, archive, or delete records
    Author(AuthorArgs),
    /// Show context for a slug, file, or folder
    Explain(ExplainArgs),
    /// Code and test materialization
    Code {
        #[command(subcommand)]
        command: CodeCommand,
    },
}

#[derive(Debug, clap::Args)]
pub struct InitArgs {
    /// Project name. Defaults to the repository directory name.
    #[arg(long)]
    pub name: Option<String>,
    /// Import a JSONL catalog when the database is missing.
    #[arg(long)]
    pub import: Option<PathBuf>,
    /// Report drift and change nothing.
    #[arg(long)]
    pub check: bool,
    /// Recreate the database from the JSONL export.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Subcommand)]
pub enum DbCommand {
    /// Run a read-only SQL query
    Query {
        /// SQL text. Mutations need --write.
        sql: String,
        /// Allow a mutation.
        #[arg(long)]
        write: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = QueryFormat::Table)]
        format: QueryFormat,
    },
    /// Export the catalog to JSONL
    Export {
        /// Output path. Defaults to .coherence/exports/db.jsonl.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Import a JSONL catalog
    Import {
        /// Input JSONL path.
        path: PathBuf,
    },
    /// List record kinds and row counts
    List {
        /// List registered projects instead of record kinds.
        #[arg(long)]
        projects: bool,
    },
    /// Set the database path, output format, and behavior
    Configure,
    /// Show project identity and database state
    Status,
    /// Apply migrations
    Migrate,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum QueryFormat {
    Table,
    Json,
    Jsonl,
}

#[derive(Debug, clap::Args)]
pub struct VerifyArgs {
    /// Targets: spec/<slug>, ac/<slug>, or file/<path>.
    pub targets: Vec<String>,
    /// Read targets from a file. One target per line.
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Print one JSON report.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, clap::Args)]
pub struct AuthorArgs {
    /// Record kind: spec, ac, owner, link, code-file, and others.
    pub kind: String,
    #[command(subcommand)]
    pub action: AuthorAction,
    /// Target slug for edit, archive, and delete.
    pub target: Option<String>,
    /// Read records or patches from JSONL. Use - for stdin.
    #[arg(long)]
    pub file: Option<String>,
    /// Do not ask for confirmation.
    #[arg(long)]
    pub yes: bool,
    /// Print planned changes and exit.
    #[arg(long)]
    pub dry_run: bool,
    /// Remove the row instead of archiving it.
    #[arg(long)]
    pub hard: bool,
}

#[derive(Debug, Subcommand)]
pub enum AuthorAction {
    /// Create a record
    Create,
    /// Edit a record
    Edit,
    /// Set status to archived
    Archive,
    /// Delete a record
    Delete,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ExplainFormat {
    Human,
    Json,
    Jsonl,
    Agent,
}

#[derive(Debug, clap::Args)]
pub struct ExplainArgs {
    /// Roots: slugs, IDs, files, or folders.
    pub targets: Vec<String>,
    /// Add comma-separated slugs as extra roots.
    #[arg(long, value_delimiter = ',')]
    pub add_slugs: Vec<String>,
    /// Add a folder as an extra root.
    #[arg(long)]
    pub add_folder: Vec<PathBuf>,
    /// Link traversal depth.
    #[arg(long, default_value_t = 1)]
    pub depth: u8,
    /// Select a field. Repeat to add fields.
    #[arg(long)]
    pub field: Vec<String>,
    /// Field preset: minimal, review, agent, or full.
    #[arg(long)]
    pub preset: Option<String>,
    /// Add record kinds: acs, code, runs, links, or extensions.
    #[arg(long, value_delimiter = ',')]
    pub include: Vec<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = ExplainFormat::Human)]
    pub format: ExplainFormat,
    /// Cap the output size.
    #[arg(long)]
    pub max_bytes: Option<usize>,
}

#[derive(Debug, Subcommand)]
pub enum CodeCommand {
    /// Create missing test files from the AC graph
    Materialize {
        #[command(subcommand)]
        action: Option<MaterializeAction>,
        /// Workspace root. Defaults to the current directory.
        #[arg(long)]
        workspace: Option<PathBuf>,
    },
    /// Ingest SCIP symbols. Stub.
    Scip,
    /// Compare the code graph at two points in time. Stub.
    Diff,
}

#[derive(Debug, Subcommand)]
pub enum MaterializeAction {
    /// Set test placement rules
    Configure,
    /// Fail when a linked test file is missing
    Check,
}

/// Parse arguments and dispatch. Returns a process exit code.
pub fn run(args: &[String]) -> i32 {
    if is_legacy(args) {
        return legacy::run(args);
    }

    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(err) => {
            let _ = err.print();
            return err.exit_code();
        }
    };

    dispatch(cli)
}

fn is_legacy(args: &[String]) -> bool {
    match args.get(1).map(String::as_str) {
        Some("db") => args
            .get(2)
            .is_some_and(|sub| LEGACY_DB_SUBCOMMANDS.contains(&sub.as_str())),
        Some(command) => LEGACY_COMMANDS.contains(&command),
        None => false,
    }
}

fn dispatch(cli: Cli) -> i32 {
    match cli.command {
        Command::Init(_) => not_implemented("init"),
        Command::Tui => match coherence_core_db_tui::run_terminal() {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("coherence tui: {err}");
                1
            }
        },
        Command::Verify(_) => not_implemented("verify"),
        Command::Author(_) => not_implemented("author"),
        Command::Explain(_) => not_implemented("explain"),
        Command::Db { command } => dispatch_db(command),
        Command::Code { command } => dispatch_code(command),
    }
}

fn dispatch_db(command: DbCommand) -> i32 {
    match command {
        DbCommand::Migrate => coherence_core_db::commands::migrate::run(),
        DbCommand::Export { out } => {
            let mut args = vec!["--env".to_string(), "dev".to_string()];
            if let Some(path) = out {
                args.push("--out".to_string());
                args.push(path.display().to_string());
            }
            coherence_core_db::commands::db_export_jsonl::run(&args)
        }
        DbCommand::Import { path } => {
            let args = vec![
                "--env".to_string(),
                "dev".to_string(),
                "--in".to_string(),
                path.display().to_string(),
                "--confirm".to_string(),
            ];
            coherence_core_db::commands::db_import_jsonl::run(&args)
        }
        DbCommand::List { projects: false } => {
            coherence_core_db::commands::db_list_databases::run()
        }
        DbCommand::List { projects: true } => not_implemented("db list --projects"),
        DbCommand::Status => coherence_core_db::commands::doctor::run(),
        DbCommand::Query { .. } | DbCommand::Configure => not_implemented("db"),
    }
}

fn dispatch_code(command: CodeCommand) -> i32 {
    match command {
        CodeCommand::Materialize { action, workspace } => {
            let sub = match action {
                None => "materialize-rust",
                Some(MaterializeAction::Check) => "check-rust",
                Some(MaterializeAction::Configure) => {
                    return not_implemented("code materialize configure");
                }
            };
            let mut args = vec![sub.to_string()];
            if let Some(path) = workspace {
                args.push("--workspace".to_string());
                args.push(path.display().to_string());
            }
            coherence_core_db::commands::ac_tests_cmd::run(&args)
        }
        CodeCommand::Scip => not_implemented("code scip"),
        CodeCommand::Diff => not_implemented("code diff"),
    }
}

fn not_implemented(command: &str) -> i32 {
    eprintln!("coherence {command}: not implemented yet (track issue #2)");
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        let mut argv = vec!["coherence".to_string()];
        argv.extend(args.iter().map(|arg| (*arg).to_string()));
        Cli::try_parse_from(argv)
    }

    #[test]
    fn parses_verify_targets() {
        let cli = parse(&["verify", "ac/session-recording/start"]).unwrap();
        assert!(matches!(cli.command, Command::Verify(_)));
    }

    #[test]
    fn parses_code_materialize_check() {
        let cli = parse(&["code", "materialize", "check"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Code {
                command: CodeCommand::Materialize {
                    action: Some(MaterializeAction::Check),
                    ..
                }
            }
        ));
    }

    #[test]
    fn rejects_unknown_command() {
        assert!(parse(&["nope"]).is_err());
    }

    #[test]
    fn routes_old_db_subcommands_to_legacy() {
        let args = vec![
            "coherence".to_string(),
            "db".to_string(),
            "list-databases".to_string(),
        ];
        assert!(is_legacy(&args));
    }

    #[test]
    fn routes_new_db_subcommands_to_clap() {
        let args = vec![
            "coherence".to_string(),
            "db".to_string(),
            "list".to_string(),
        ];
        assert!(!is_legacy(&args));
    }
}
