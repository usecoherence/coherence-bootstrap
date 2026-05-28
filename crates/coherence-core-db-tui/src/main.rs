mod action;
mod app;
mod effects;
mod project_discovery;
mod repository;
mod theme;
mod tree;
mod update;
mod ui;

use ratatui::crossterm::event::{self, Event};

use action::{key_to_action, AppAction};
use update::update;
use theme::THEME;

fn main() -> Result<(), String> {
    let projects = project_discovery::discover_projects();
    if projects.is_empty() {
        eprintln!("No Coherence projects found under ~/git/");
        eprintln!("(Looking for ~/git/**/*/.coherence/project.toml with find -maxdepth 6)");
        eprintln!("Try: find ~/git -name project.toml -path '*/.coherence/project.toml'");
        return Ok(());
    }

    let mut app = app::AppState::new(projects);
    let terminal = ratatui::init();
    let result = run(terminal, &mut app);
    ratatui::restore();
    result
}

fn run(mut terminal: ratatui::DefaultTerminal, app: &mut app::AppState) -> Result<(), String> {
    loop {
        terminal
            .draw(|frame| ui::ui(frame, app, &THEME))
            .map_err(|e| format!("draw: {e}"))?;

        if !event::poll(std::time::Duration::from_millis(100))
            .map_err(|e| format!("poll: {e}"))?
        {
            continue;
        }

        let Event::Key(key) = event::read().map_err(|e| format!("read: {e}"))? else {
            continue;
        };

        let Some(action) = key_to_action(key, app) else {
            continue;
        };

        if matches!(action, AppAction::Quit) {
            break Ok(());
        }

        let effects = update(app, action);
        effects::execute_effects(app, effects);
    }
}
