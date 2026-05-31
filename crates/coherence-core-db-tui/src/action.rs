use ratatui::crossterm::event::{KeyCode, KeyEventKind};

use crate::app::{AppState, Screen};

#[derive(Debug, Clone, PartialEq)]
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
    SaveDraft,
    SwitchToProjectPicker,
    SwitchToEnvPicker,
    VerifySelected,
    VerifyAll,
}

pub fn key_to_action(
    key: ratatui::crossterm::event::KeyEvent,
    app: &AppState,
) -> Option<AppAction> {
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
                    KeyCode::Enter => Some(AppAction::SaveDraft),
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
                    KeyCode::Char('v') => Some(AppAction::VerifySelected),
                    KeyCode::Char('V') => Some(AppAction::VerifyAll),
                    KeyCode::Char('q') => Some(AppAction::Quit),
                    _ => None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyModifiers;

    use super::*;

    fn make_app_state() -> AppState {
        AppState {
            edit_mode: false,
            draft: None,
            screen: Screen::Specs,
            focus_tree: true,
            detail_scroll: 0,
            projects: Vec::new(),
            selected_project: 0,
            envs: Vec::new(),
            selected_env: 0,
            graph: None,
            tree_items: Vec::new(),
            selected_tree: 0,
            tree_scroll: 0,
            detail_spec_id: None,
            detail_ac_id: None,
            verification_statuses: std::collections::HashMap::new(),
            status: String::new(),
            project_dir: None,
            repo: None,
        }
    }

    #[test]
    fn key_enter_in_edit_mode_returns_save_draft() {
        let mut app = make_app_state();
        app.edit_mode = true;
        let key = ratatui::crossterm::event::KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(key_to_action(key, &app), Some(AppAction::SaveDraft));
    }

    #[test]
    fn key_esc_in_edit_mode_returns_back() {
        let mut app = make_app_state();
        app.edit_mode = true;
        let key = ratatui::crossterm::event::KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(key_to_action(key, &app), Some(AppAction::Back));
    }

    #[test]
    fn key_v_in_specs_returns_verify_selected() {
        let app = make_app_state();
        let key = ratatui::crossterm::event::KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
        assert_eq!(key_to_action(key, &app), Some(AppAction::VerifySelected));
    }

    #[test]
    fn key_shift_v_in_specs_returns_verify_all() {
        let app = make_app_state();
        let key = ratatui::crossterm::event::KeyEvent::new(KeyCode::Char('V'), KeyModifiers::SHIFT);
        assert_eq!(key_to_action(key, &app), Some(AppAction::VerifyAll));
    }
}
