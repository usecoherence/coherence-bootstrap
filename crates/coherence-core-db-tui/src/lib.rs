#![allow(
    clippy::assigning_clones,
    clippy::expect_used,
    clippy::format_push_string,
    clippy::ignored_unit_patterns,
    clippy::implicit_clone,
    clippy::let_unit_value,
    clippy::manual_let_else,
    clippy::manual_string_new,
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::match_single_binding,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::needless_borrow,
    clippy::needless_pass_by_value,
    clippy::redundant_clone,
    clippy::single_match_else,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::wildcard_enum_match_arm,
)]

pub mod action;
pub mod app;
pub mod edit;
pub mod effects;
pub mod project_discovery;
pub mod repository;
pub mod theme;
pub mod tree;
pub mod update;
pub mod ui;

pub use action::{key_to_action, AppAction};
pub use app::{AppState, Screen};
pub use edit::Draft;
pub use effects::Effect;
pub use update::update;

pub fn run_terminal() -> Result<(), String> {
    let projects = project_discovery::discover_projects();
    if projects.is_empty() {
        eprintln!("No Coherence projects found under ~/git/");
        eprintln!("(Looking for ~/git/**/*/.coherence/project.toml with find -maxdepth 6)");
        eprintln!("Try: find ~/git -name project.toml -path '*/.coherence/project.toml'");
        return Ok(());
    }

    let mut app = app::AppState::new(projects);
    let terminal = ratatui::init();
    let result = run_loop(terminal, &mut app);
    ratatui::restore();
    result
}

fn run_loop(mut terminal: ratatui::DefaultTerminal, app: &mut app::AppState) -> Result<(), String> {
    use ratatui::crossterm::event::{self, Event};

    loop {
        terminal
            .draw(|frame| ui::ui(frame, app, &theme::THEME))
            .map_err(|e| format!("draw: {e}"))?;

        if !event::poll(std::time::Duration::from_millis(100))
            .map_err(|e| format!("poll: {e}"))?
        {
            continue;
        }

        let Event::Key(key) = event::read().map_err(|e| format!("read: {e}"))? else {
            continue;
        };

        let Some(action) = action::key_to_action(key, app) else {
            continue;
        };

        if matches!(action, action::AppAction::Quit) {
            break Ok(());
        }

        let effects = update::update(app, action);
        effects::execute_effects(app, effects);
    }
}
