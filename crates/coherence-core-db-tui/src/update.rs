#![allow(clippy::wildcard_enum_match_arm)]
use coherence_core_db::models::{ReviewMode, RiskLevel, SpecLevel, SpecStatus};

use crate::action::AppAction;
use crate::app::{AppState, Screen};
use crate::edit::Draft;
use crate::effects::Effect;

pub fn update(app: &mut AppState, action: AppAction) -> Vec<Effect> {
    match action {
        AppAction::NavUp => nav_up(app),
        AppAction::NavDown => nav_down(app),
        AppAction::Enter => handle_enter(app),
        AppAction::Back => handle_back(app),

        AppAction::Quit => {
            app.status = "quit".into();
            vec![]
        }

        AppAction::ToggleExpand => {
            app.toggle_expand();
            vec![]
        }

        AppAction::FocusTreeUp => {
            app.selected_tree = app.selected_tree.saturating_sub(1);
            app.update_preview();
            vec![]
        }

        AppAction::FocusTreeDown => {
            app.selected_tree = (app.selected_tree + 1).min(app.tree_items.len().saturating_sub(1));
            app.update_preview();
            vec![]
        }

        AppAction::FocusTreeLeft => focus_tree_left(app),

        AppAction::FocusDetail => {
            app.focus_tree = true;
            vec![]
        }

        AppAction::EnterEditMode => enter_edit_mode(app),
        AppAction::CycleStatus => cycle_status(app),
        AppAction::CycleLevel => cycle_level(app),
        AppAction::CycleReviewMode => cycle_review_mode(app),
        AppAction::CycleRiskLevel => cycle_risk_level(app),
        AppAction::OpenEditor => open_editor(app),
        AppAction::SaveDraft => save_draft(app),

        AppAction::SwitchToProjectPicker => {
            app.screen = Screen::ProjectPicker;
            app.status = "Select a project".into();
            vec![]
        }

        AppAction::SwitchToEnvPicker => {
            app.screen = Screen::EnvPicker;
            app.status = format!(
                "Select environment for {}",
                app.projects[app.selected_project].1
            );
            vec![]
        }

        AppAction::VerifySelected => verify_selected(app),
        AppAction::VerifyAll => vec![Effect::VerifyAll],
    }
}

fn nav_up(app: &mut AppState) -> Vec<Effect> {
    match app.screen {
        Screen::ProjectPicker => {
            app.selected_project = app.selected_project.saturating_sub(1);
        }
        Screen::EnvPicker => {
            app.selected_env = app.selected_env.saturating_sub(1);
        }
        Screen::Specs if !app.focus_tree => {
            app.detail_scroll = app.detail_scroll.saturating_sub(1);
        }
        Screen::Specs => {}
    }
    vec![]
}

fn nav_down(app: &mut AppState) -> Vec<Effect> {
    match app.screen {
        Screen::ProjectPicker => {
            app.selected_project = (app.selected_project + 1).min(app.projects.len() - 1);
        }
        Screen::EnvPicker => {
            app.selected_env = (app.selected_env + 1).min(app.envs.len() - 1);
        }
        Screen::Specs if !app.focus_tree => {
            app.detail_scroll = app.detail_scroll.saturating_add(1);
        }
        Screen::Specs => {}
    }
    vec![]
}

fn handle_enter(app: &mut AppState) -> Vec<Effect> {
    match app.screen {
        Screen::ProjectPicker => {
            app.screen = Screen::EnvPicker;
            app.status = format!(
                "Select environment for {}",
                app.projects[app.selected_project].1
            );
            vec![]
        }
        Screen::EnvPicker => {
            app.screen = Screen::Specs;
            app.focus_tree = true;
            vec![Effect::RefreshGraph]
        }
        Screen::Specs if app.focus_tree => {
            if app.tree_items[app.selected_tree].has_children {
                app.toggle_expand();
                vec![]
            } else {
                app.focus_tree = false;
                vec![]
            }
        }
        Screen::Specs => vec![],
    }
}

fn handle_back(app: &mut AppState) -> Vec<Effect> {
    match app.screen {
        Screen::EnvPicker => {
            app.screen = Screen::ProjectPicker;
            vec![]
        }
        Screen::Specs if !app.focus_tree => {
            app.focus_tree = true;
            vec![]
        }
        Screen::Specs if app.edit_mode => {
            app.edit_mode = false;
            app.draft = None;
            app.status = "Edit cancelled".into();
            vec![]
        }
        Screen::Specs if app.focus_tree => {
            app.screen = Screen::EnvPicker;
            app.status = format!(
                "Select environment for {}",
                app.projects[app.selected_project].1
            );
            vec![]
        }
        _ => vec![],
    }
}

fn focus_tree_left(app: &mut AppState) -> Vec<Effect> {
    if app.tree_items.is_empty() || app.selected_tree >= app.tree_items.len() {
        return vec![];
    }
    let cur_indent = app.tree_items[app.selected_tree].indent;
    if cur_indent == 0 {
        return vec![];
    }
    for i in (0..app.selected_tree).rev() {
        if app.tree_items[i].indent < cur_indent {
            app.selected_tree = i;
            app.update_preview();
            break;
        }
    }
    vec![]
}

fn enter_edit_mode(app: &mut AppState) -> Vec<Effect> {
    app.edit_mode = true;
    let draft = if let Some(sid) = app.detail_spec_id.clone() {
        app.graph
            .as_ref()
            .and_then(|g| g.specs.iter().find(|s| s.id == sid))
            .map(Draft::from_spec)
    } else if let Some(aid) = app.detail_ac_id.clone() {
        app.graph
            .as_ref()
            .and_then(|g| g.acceptance_criteria.iter().find(|a| a.id == aid))
            .map(Draft::from_ac)
    } else {
        None
    };
    match draft {
        Some(d) => {
            app.draft = Some(d);
            app.status = "Edit: [s] status  [l] level  [r] review  [k] risk  [e] content  [Enter] save  [Esc] cancel".into();
        }
        None => {
            app.status = "Nothing selected to edit".into();
        }
    }
    vec![]
}

fn cycle_status(app: &mut AppState) -> Vec<Effect> {
    let Some(draft) = active_draft_mut(app) else {
        return vec![];
    };
    match draft {
        Draft::Spec { pending_status, .. } => {
            *pending_status = match pending_status {
                SpecStatus::Draft => SpecStatus::Active,
                SpecStatus::Active => SpecStatus::Deprecated,
                SpecStatus::Deprecated => SpecStatus::Archived,
                SpecStatus::Archived => SpecStatus::Draft,
            };
            app.status = format!("Status → {}", pending_status.as_db_str());
        }
        Draft::Ac { .. } => app.status = "Spec not selected for status edit".into(),
    }
    vec![]
}

fn cycle_level(app: &mut AppState) -> Vec<Effect> {
    let Some(draft) = active_draft_mut(app) else {
        return vec![];
    };
    match draft {
        Draft::Spec { pending_level, .. } => {
            *pending_level = match pending_level {
                SpecLevel::Product => SpecLevel::System,
                SpecLevel::System => SpecLevel::Module,
                SpecLevel::Module => SpecLevel::Component,
                SpecLevel::Component => SpecLevel::Foundation,
                SpecLevel::Foundation => SpecLevel::Product,
            };
            app.status = format!("Level → {}", pending_level.as_db_str());
        }
        Draft::Ac { .. } => app.status = "Spec not selected for level edit".into(),
    }
    vec![]
}

fn cycle_review_mode(app: &mut AppState) -> Vec<Effect> {
    let Some(draft) = active_draft_mut(app) else {
        return vec![];
    };
    match draft {
        Draft::Ac {
            pending_review_mode,
            ..
        } => {
            *pending_review_mode = match pending_review_mode {
                ReviewMode::Manual => ReviewMode::Automated,
                ReviewMode::Automated => ReviewMode::Hybrid,
                ReviewMode::Hybrid => ReviewMode::Manual,
            };
            app.status = format!("Review → {}", pending_review_mode.as_db_str());
        }
        Draft::Spec { .. } => app.status = "AC not selected for review edit".into(),
    }
    vec![]
}

fn cycle_risk_level(app: &mut AppState) -> Vec<Effect> {
    let Some(draft) = active_draft_mut(app) else {
        return vec![];
    };
    match draft {
        Draft::Ac {
            pending_risk_level, ..
        } => {
            *pending_risk_level = match pending_risk_level {
                RiskLevel::Low => RiskLevel::Medium,
                RiskLevel::Medium => RiskLevel::High,
                RiskLevel::High => RiskLevel::Critical,
                RiskLevel::Critical => RiskLevel::Low,
            };
            app.status = format!("Risk → {}", pending_risk_level.as_db_str());
        }
        Draft::Spec { .. } => app.status = "AC not selected for risk edit".into(),
    }
    vec![]
}

fn open_editor(app: &mut AppState) -> Vec<Effect> {
    if app.draft.is_none() {
        app.status = "No active draft".into();
        return vec![];
    }
    match app.draft.as_ref() {
        Some(Draft::Spec { spec_id, .. }) => vec![Effect::OpenEditorForSpec(spec_id.clone())],
        Some(Draft::Ac { ac_id, .. }) => vec![Effect::OpenEditorForAc(ac_id.clone())],
        None => unreachable!(),
    }
}

fn save_draft(app: &mut AppState) -> Vec<Effect> {
    let Some(draft) = app.draft.take() else {
        app.status = "No active draft".into();
        return vec![];
    };
    if let Err(errors) = draft.validate() {
        app.draft = Some(draft);
        app.status = format!("Validation errors: {}", errors.join("; "));
        return vec![];
    }
    if !draft.is_dirty() {
        app.edit_mode = false;
        app.status = "No changes to save".into();
        return vec![];
    }
    let Some(effects) = effects_for_saved_draft(app, &draft) else {
        return vec![];
    };
    app.edit_mode = false;
    app.status = "Draft saved".into();
    effects
}

fn active_draft_mut(app: &mut AppState) -> Option<&mut Draft> {
    if app.draft.is_none() {
        app.status = "No active draft".into();
    }
    app.draft.as_mut()
}

fn effects_for_saved_draft(app: &mut AppState, draft: &Draft) -> Option<Vec<Effect>> {
    match draft {
        Draft::Spec {
            spec_id,
            pending_status,
            pending_level,
            pending_description,
            ..
        } => save_entity(
            app,
            "Spec not found in graph",
            |app| {
                let Some(graph) = app.graph.as_mut() else {
                    return;
                };
                let Some(spec) = graph.specs.iter_mut().find(|s| s.id == *spec_id) else {
                    return;
                };
                spec.status = *pending_status;
                spec.level = *pending_level;
                if let Some(desc) = pending_description {
                    spec.description = desc.clone();
                }
            },
            |app| {
                app.graph
                    .as_ref()
                    .and_then(|g| g.specs.iter().find(|s| s.id == *spec_id))
                    .cloned()
            },
            Effect::PersistSpec,
        ),
        Draft::Ac {
            ac_id,
            pending_review_mode,
            pending_risk_level,
            pending_intent,
            ..
        } => save_entity(
            app,
            "AC not found in graph",
            |app| {
                let Some(graph) = app.graph.as_mut() else {
                    return;
                };
                let Some(ac) = graph
                    .acceptance_criteria
                    .iter_mut()
                    .find(|a| a.id == *ac_id)
                else {
                    return;
                };
                ac.review_mode = *pending_review_mode;
                ac.risk_level = *pending_risk_level;
                if let Some(intent) = pending_intent {
                    ac.intent = intent.clone();
                }
            },
            |app| {
                app.graph
                    .as_ref()
                    .and_then(|g| g.acceptance_criteria.iter().find(|a| a.id == *ac_id))
                    .cloned()
            },
            Effect::PersistAc,
        ),
    }
}

fn save_entity<T>(
    app: &mut AppState,
    missing_status: &str,
    update_graph: impl FnOnce(&mut AppState),
    load_saved: impl FnOnce(&AppState) -> Option<T>,
    effect: impl FnOnce(T) -> Effect,
) -> Option<Vec<Effect>> {
    update_graph(app);
    load_saved(app)
        .map(|entity| vec![effect(entity)])
        .or_else(|| {
            app.status = missing_status.into();
            None
        })
}

fn verify_selected(app: &mut AppState) -> Vec<Effect> {
    let Some(item) = app.tree_items.get(app.selected_tree) else {
        app.status = "Nothing selected to verify".into();
        return vec![];
    };

    if item.parent_spec_id.is_some() {
        return vec![Effect::VerifyAc(item.id.clone())];
    }
    if item.is_spec {
        return vec![Effect::VerifySpec(item.id.clone())];
    }
    let Some(level) = level_from_tree_label(&item.label) else {
        app.status = "Selected row cannot be verified".into();
        return vec![];
    };
    vec![Effect::VerifyLevel(level)]
}

fn level_from_tree_label(label: &str) -> Option<SpecLevel> {
    match label {
        "Product" => Some(SpecLevel::Product),
        "System" => Some(SpecLevel::System),
        "Module" => Some(SpecLevel::Module),
        "Component" => Some(SpecLevel::Component),
        "Foundation" => Some(SpecLevel::Foundation),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::TreeItem;

    fn app_with_tree(item: TreeItem) -> AppState {
        let mut app = AppState::new(vec![]);
        app.screen = Screen::Specs;
        app.tree_items = vec![item];
        app
    }

    #[test]
    fn verify_selected_dispatches_ac_effect() {
        let mut app = app_with_tree(TreeItem {
            indent: 2,
            label: "ac".into(),
            id: "AC1".into(),
            is_spec: false,
            expanded: false,
            has_children: false,
            parent_spec_id: Some("S1".into()),
        });

        let effects = update(&mut app, AppAction::VerifySelected);

        assert_eq!(effects, vec![Effect::VerifyAc("AC1".into())]);
    }

    #[test]
    fn verify_selected_dispatches_spec_effect() {
        let mut app = app_with_tree(TreeItem {
            indent: 1,
            label: "spec".into(),
            id: "S1".into(),
            is_spec: true,
            expanded: false,
            has_children: true,
            parent_spec_id: None,
        });

        let effects = update(&mut app, AppAction::VerifySelected);

        assert_eq!(effects, vec![Effect::VerifySpec("S1".into())]);
    }

    #[test]
    fn verify_selected_dispatches_level_effect() {
        let mut app = app_with_tree(TreeItem {
            indent: 0,
            label: "Product".into(),
            id: String::new(),
            is_spec: false,
            expanded: false,
            has_children: true,
            parent_spec_id: None,
        });

        let effects = update(&mut app, AppAction::VerifySelected);

        assert_eq!(effects, vec![Effect::VerifyLevel(SpecLevel::Product)]);
    }

    #[test]
    fn verify_all_dispatches_all_effect() {
        let mut app = AppState::new(vec![]);

        let effects = update(&mut app, AppAction::VerifyAll);

        assert_eq!(effects, vec![Effect::VerifyAll]);
    }

    #[test]
    fn verify_selected_without_tree_item_reports_status() {
        let mut app = AppState::new(vec![]);

        let effects = update(&mut app, AppAction::VerifySelected);

        assert!(effects.is_empty());
        assert_eq!(app.status, "Nothing selected to verify");
    }
}
