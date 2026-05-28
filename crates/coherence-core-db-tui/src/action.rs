use ratatui::crossterm::event::{KeyCode, KeyEventKind};

use crate::app::{AppState, Screen};

pub enum AppAction {
    NavUp,
    NavDown,
    Enter,
    Back,
    Quit,
    ToggleExpand,
    FocusTreeUp,
    FocusTreeDown,
    FocusTreeLeft,
    FocusDetail,
    EnterEditMode,
    CycleStatus,
    CycleLevel,
    CycleReviewMode,
    CycleRiskLevel,
    OpenEditor,
    SwitchToProjectPicker,
    SwitchToEnvPicker,
}

pub fn key_to_action(key: ratatui::crossterm::event::KeyEvent, app: &AppState) -> Option<AppAction> {
    if key.kind != KeyEventKind::Press {
        return None;
    }

    match app.screen {
        Screen::ProjectPicker => match key.code {
            KeyCode::Up => Some(AppAction::NavUp),
            KeyCode::Down => Some(AppAction::NavDown),
            KeyCode::Enter => Some(AppAction::Enter),
            KeyCode::Esc => Some(AppAction::Back),
            KeyCode::Char('q') => Some(AppAction::Quit),
            _ => None,
        },
        Screen::EnvPicker => match key.code {
            KeyCode::Up => Some(AppAction::NavUp),
            KeyCode::Down => Some(AppAction::NavDown),
            KeyCode::Enter => Some(AppAction::Enter),
            KeyCode::Esc => Some(AppAction::Back),
            KeyCode::Char('q') => Some(AppAction::Quit),
            _ => None,
        },
        Screen::Specs => {
            if app.edit_mode {
                match key.code {
                    KeyCode::Char('e') => Some(AppAction::OpenEditor),
                    KeyCode::Char('s') => Some(AppAction::CycleStatus),
                    KeyCode::Char('l') => Some(AppAction::CycleLevel),
                    KeyCode::Char('r') => Some(AppAction::CycleReviewMode),
                    KeyCode::Char('k') => Some(AppAction::CycleRiskLevel),
                    KeyCode::Esc => Some(AppAction::Back),
                    _ => None,
                }
            } else {
                match key.code {
                    KeyCode::Up if app.focus_tree => Some(AppAction::FocusTreeUp),
                    KeyCode::Down if app.focus_tree => Some(AppAction::FocusTreeDown),
                    KeyCode::Enter if app.focus_tree => Some(AppAction::ToggleExpand),
                    KeyCode::Up if !app.focus_tree => Some(AppAction::NavUp),
                    KeyCode::Down if !app.focus_tree => Some(AppAction::NavDown),
                    KeyCode::Enter if !app.focus_tree => Some(AppAction::FocusDetail),
                    KeyCode::Esc if !app.focus_tree => Some(AppAction::FocusDetail),
                    KeyCode::Esc if app.focus_tree => Some(AppAction::Back),
                    KeyCode::Left if app.focus_tree => Some(AppAction::FocusTreeLeft),
                    KeyCode::Char('e') => Some(AppAction::EnterEditMode),
                    KeyCode::Char('p') => Some(AppAction::SwitchToProjectPicker),
                    KeyCode::Char('d') => Some(AppAction::SwitchToEnvPicker),
                    KeyCode::Char('q') => Some(AppAction::Quit),
                    _ => None,
                }
            }
        }
    }
}
