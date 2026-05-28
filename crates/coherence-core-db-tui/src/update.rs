use coherence_core_db::models::{ReviewMode, RiskLevel, SpecStatus, SpecLevel};

use crate::action::AppAction;
use crate::app::{AppState, Screen};
use crate::effects::Effect;

pub fn update(app: &mut AppState, action: AppAction) -> Vec<Effect> {
    match action {
        AppAction::NavUp => match app.screen {
            Screen::ProjectPicker => {
                app.selected_project = app.selected_project.saturating_sub(1);
                vec![]
            }
            Screen::EnvPicker => {
                app.selected_env = app.selected_env.saturating_sub(1);
                vec![]
            }
            Screen::Specs if !app.focus_tree => {
                app.detail_scroll = app.detail_scroll.saturating_sub(1);
                vec![]
            }
            _ => vec![],
        },

        AppAction::NavDown => match app.screen {
            Screen::ProjectPicker => {
                app.selected_project = (app.selected_project + 1).min(app.projects.len() - 1);
                vec![]
            }
            Screen::EnvPicker => {
                app.selected_env = (app.selected_env + 1).min(app.envs.len() - 1);
                vec![]
            }
            Screen::Specs if !app.focus_tree => {
                app.detail_scroll = app.detail_scroll.saturating_add(1);
                vec![]
            }
            _ => vec![],
        },

        AppAction::Enter => match app.screen {
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
                let item = &app.tree_items[app.selected_tree].clone();
                if item.has_children {
                    app.toggle_expand();
                    vec![]
                } else {
                    app.focus_tree = false;
                    vec![]
                }
            }
            _ => vec![],
        },

        AppAction::Back => match app.screen {
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
                app.status = "Edit mode closed".into();
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
        },

        AppAction::Quit => {
            app.status = "quit".into();
            vec![] // handled by event loop via separate flag
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
            app.selected_tree = (app.selected_tree + 1)
                .min(app.tree_items.len().saturating_sub(1));
            app.update_preview();
            vec![]
        }

        AppAction::FocusTreeLeft => {
            if !app.tree_items.is_empty() && app.selected_tree < app.tree_items.len() {
                let cur_indent = app.tree_items[app.selected_tree].indent;
                if cur_indent > 0 {
                    for i in (0..app.selected_tree).rev() {
                        if app.tree_items[i].indent < cur_indent {
                            app.selected_tree = i;
                            app.update_preview();
                            break;
                        }
                    }
                }
            }
            vec![]
        }

        AppAction::FocusDetail => {
            app.focus_tree = true;
            vec![]
        }

        AppAction::EnterEditMode => {
            app.edit_mode = true;
            app.status = "Edit mode: [s] status  [l] level  [r] review  [k] risk  [e] content  [Esc] exit".into();
            vec![]
        }

        AppAction::CycleStatus => {
            let sid = match app.detail_spec_id.clone() {
                Some(id) => id,
                None => {
                    app.status = "No spec selected".into();
                    return vec![];
                }
            };
            match app.graph.as_ref().and_then(|g| g.specs.iter().find(|s| s.id == sid)) {
                Some(spec) => {
                    let next = match spec.status {
                        SpecStatus::Draft => SpecStatus::Active,
                        SpecStatus::Active => SpecStatus::Deprecated,
                        SpecStatus::Deprecated => SpecStatus::Archived,
                        SpecStatus::Archived => SpecStatus::Draft,
                    };
                    let mut updated = spec.clone();
                    updated.status = next;
                    vec![Effect::PersistSpec(updated)]
                }
                None => {
                    app.status = "spec not found".into();
                    vec![]
                }
            }
        }

        AppAction::CycleLevel => {
            let sid = match app.detail_spec_id.clone() {
                Some(id) => id,
                None => {
                    app.status = "No spec selected".into();
                    return vec![];
                }
            };
            match app.graph.as_ref().and_then(|g| g.specs.iter().find(|s| s.id == sid)) {
                Some(spec) => {
                    let next = match spec.level {
                        SpecLevel::Product => SpecLevel::System,
                        SpecLevel::System => SpecLevel::Module,
                        SpecLevel::Module => SpecLevel::Product,
                    };
                    let mut updated = spec.clone();
                    updated.level = next;
                    vec![Effect::PersistSpec(updated)]
                }
                None => {
                    app.status = "spec not found".into();
                    vec![]
                }
            }
        }

        AppAction::CycleReviewMode => {
            let aid = match app.detail_ac_id.clone() {
                Some(id) => id,
                None => {
                    app.status = "No AC selected".into();
                    return vec![];
                }
            };
            match app.graph.as_ref().and_then(|g| g.acceptance_criteria.iter().find(|a| a.id == aid)) {
                Some(ac) => {
                    let next = match ac.review_mode {
                        ReviewMode::Manual => ReviewMode::Automated,
                        ReviewMode::Automated => ReviewMode::Hybrid,
                        ReviewMode::Hybrid => ReviewMode::Manual,
                    };
                    let mut updated = ac.clone();
                    updated.review_mode = next;
                    vec![Effect::PersistAc(updated)]
                }
                None => {
                    app.status = "AC not found".into();
                    vec![]
                }
            }
        }

        AppAction::CycleRiskLevel => {
            let aid = match app.detail_ac_id.clone() {
                Some(id) => id,
                None => {
                    app.status = "No AC selected".into();
                    return vec![];
                }
            };
            match app.graph.as_ref().and_then(|g| g.acceptance_criteria.iter().find(|a| a.id == aid)) {
                Some(ac) => {
                    let next = match ac.risk_level {
                        RiskLevel::Low => RiskLevel::Medium,
                        RiskLevel::Medium => RiskLevel::High,
                        RiskLevel::High => RiskLevel::Critical,
                        RiskLevel::Critical => RiskLevel::Low,
                    };
                    let mut updated = ac.clone();
                    updated.risk_level = next;
                    vec![Effect::PersistAc(updated)]
                }
                None => {
                    app.status = "AC not found".into();
                    vec![]
                }
            }
        }

        AppAction::OpenEditor => {
            if let Some(sid) = app.detail_spec_id.clone() {
                vec![Effect::OpenEditorForSpec(sid)]
            } else if let Some(aid) = app.detail_ac_id.clone() {
                vec![Effect::OpenEditorForAc(aid)]
            } else {
                app.status = "Nothing selected to edit".into();
                vec![]
            }
        }

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
    }
}
