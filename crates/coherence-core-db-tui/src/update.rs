#![allow(clippy::wildcard_enum_match_arm)]
use coherence_core_db::models::{ReviewMode, RiskLevel, SpecStatus, SpecLevel};

use crate::action::AppAction;
use crate::app::{AppState, Screen};
use crate::edit::Draft;
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
            Screen::Specs => vec![],
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
            Screen::Specs => vec![],
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
            Screen::Specs => vec![],
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
            let draft = if let Some(sid) = app.detail_spec_id.clone() {
                app.graph.as_ref()
                    .and_then(|g| g.specs.iter().find(|s| s.id == sid))
                    .map(Draft::from_spec)
            } else if let Some(aid) = app.detail_ac_id.clone() {
                app.graph.as_ref()
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

        AppAction::CycleStatus => {
            let Some(ref mut draft) = app.draft else {
                app.status = "No active draft".into();
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

        AppAction::CycleLevel => {
            let Some(ref mut draft) = app.draft else {
                app.status = "No active draft".into();
                return vec![];
            };
            match draft {
                Draft::Spec { pending_level, .. } => {
                    *pending_level = match pending_level {
                        SpecLevel::Product => SpecLevel::System,
                        SpecLevel::System => SpecLevel::Module,
                        SpecLevel::Module => SpecLevel::Product,
                    };
                    app.status = format!("Level → {}", pending_level.as_db_str());
                }
                Draft::Ac { .. } => app.status = "Spec not selected for level edit".into(),
            }
            vec![]
        }

        AppAction::CycleReviewMode => {
            let Some(ref mut draft) = app.draft else {
                app.status = "No active draft".into();
                return vec![];
            };
            match draft {
                Draft::Ac { pending_review_mode, .. } => {
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

        AppAction::CycleRiskLevel => {
            let Some(ref mut draft) = app.draft else {
                app.status = "No active draft".into();
                return vec![];
            };
            match draft {
                Draft::Ac { pending_risk_level, .. } => {
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

        AppAction::OpenEditor => {
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

        AppAction::SaveDraft => {
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
            let effects = match &draft {
                Draft::Spec { spec_id, pending_status, pending_level, pending_description, .. } => {
                    if let Some(ref mut graph) = app.graph {
                        if let Some(s) = graph.specs.iter_mut().find(|s| s.id == *spec_id) {
                            s.status = *pending_status;
                            s.level = *pending_level;
                            if let Some(desc) = pending_description {
                                s.description = desc.clone();
                            }
                        }
                    }
                    let spec = match app.graph.as_ref().and_then(|g| g.specs.iter().find(|s| s.id == *spec_id)) {
                        Some(s) => s.clone(),
                        None => { app.status = "Spec not found in graph".into(); return vec![]; }
                    };
                    vec![Effect::PersistSpec(spec)]
                }
                Draft::Ac { ac_id, pending_review_mode, pending_risk_level, pending_intent, .. } => {
                    if let Some(ref mut graph) = app.graph {
                        if let Some(a) = graph.acceptance_criteria.iter_mut().find(|a| a.id == *ac_id) {
                            a.review_mode = *pending_review_mode;
                            a.risk_level = *pending_risk_level;
                            if let Some(intent) = pending_intent {
                                a.intent = intent.clone();
                            }
                        }
                    }
                    let ac = match app.graph.as_ref().and_then(|g| g.acceptance_criteria.iter().find(|a| a.id == *ac_id)) {
                        Some(a) => a.clone(),
                        None => { app.status = "AC not found in graph".into(); return vec![]; }
                    };
                    vec![Effect::PersistAc(ac)]
                }
            };
            app.edit_mode = false;
            app.status = "Draft saved".into();
            effects
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
