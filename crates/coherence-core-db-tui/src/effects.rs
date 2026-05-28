use std::env;
use std::process::Command;

use coherence_core_db::models::{AcceptanceCriterion, Spec};

use crate::app::AppState;
use crate::repository::{DoltSpecRepository, SpecRepository};

pub enum Effect {
    PersistSpec(Spec),
    PersistAc(AcceptanceCriterion),
    OpenEditorForSpec(String),
    OpenEditorForAc(String),
    RefreshGraph,
}

pub fn execute_effects(app: &mut AppState, effects: Vec<Effect>) {
    for effect in effects {
        match effect {
            Effect::PersistSpec(spec) => persist_spec(app, spec),
            Effect::PersistAc(ac) => persist_ac(app, ac),
            Effect::OpenEditorForSpec(spec_id) => open_editor_for_spec(app, &spec_id),
            Effect::OpenEditorForAc(ac_id) => open_editor_for_ac(app, &ac_id),
            Effect::RefreshGraph => refresh_graph(app),
        }
    }
}

fn repo(app: &mut AppState) -> &mut dyn SpecRepository {
    app.repo.as_mut().expect("SpecRepository not initialized").as_mut()
}

fn persist_spec(app: &mut AppState, spec: Spec) {
    match repo(app).put_spec(&spec) {
        Ok(()) => {
            app.status = format!("Status → {}", spec.status.as_db_str());
            if let Some(ref graph) = app.graph {
                let mut g = graph.clone();
                if let Some(s) = g.specs.iter_mut().find(|s| s.id == spec.id) {
                    s.status = spec.status;
                    s.level = spec.level;
                }
                app.graph = Some(g);
            }
        }
        Err(e) => app.status = format!("update failed: {e}"),
    }
}

fn persist_ac(app: &mut AppState, ac: AcceptanceCriterion) {
    let field = match ac.review_mode.as_db_str() {
        "manual" | "automated" | "hybrid" => format!("Review → {}", ac.review_mode.as_db_str()),
        _ => format!("Risk → {}", ac.risk_level.as_db_str()),
    };
    match repo(app).put_acceptance_criterion(&ac) {
        Ok(()) => {
            app.status = field;
            if let Some(ref graph) = app.graph {
                let mut g = graph.clone();
                if let Some(a) = g.acceptance_criteria.iter_mut().find(|a| a.id == ac.id) {
                    a.review_mode = ac.review_mode;
                    a.risk_level = ac.risk_level;
                }
                app.graph = Some(g);
            }
        }
        Err(e) => app.status = format!("update failed: {e}"),
    }
}

fn open_editor_for_spec(app: &mut AppState, spec_id: &str) {
    let spec = match repo(app).get_spec(spec_id) {
        Ok(Some(s)) => s,
        Ok(None) => { app.status = "spec not found".into(); return; }
        Err(e) => { app.status = format!("get spec: {e}"); return; }
    };

    let tmp = format!("/tmp/coherence-spec-{}.md", spec.id);
    if std::fs::write(&tmp, &spec.description).is_err() {
        app.status = "write failed".into();
        return;
    }

    let editor = env::var("EDITOR").or_else(|_| env::var("VISUAL")).unwrap_or_else(|_| "micro".to_string());
    let ok = Command::new(&editor).arg(&tmp).status().map(|s| s.success()).unwrap_or(false);

    if ok {
        let new_desc = std::fs::read_to_string(&tmp).unwrap_or_default();
        let mut updated = spec.clone();
        updated.description = new_desc;
        match repo(app).put_spec(&updated) {
            Ok(()) => {
                app.status = "Spec description updated".into();
                if let Some(ref graph) = app.graph {
                    let mut g = graph.clone();
                    if let Some(s) = g.specs.iter_mut().find(|s| s.id == updated.id) {
                        s.description = updated.description.clone();
                    }
                    app.graph = Some(g);
                }
            }
            Err(e) => app.status = format!("update failed: {e}"),
        }
    } else {
        app.status = "Edit cancelled".into();
    }
    let _ = std::fs::remove_file(&tmp);
}

fn open_editor_for_ac(app: &mut AppState, ac_id: &str) {
    let ac = match repo(app).get_acceptance_criterion(ac_id) {
        Ok(Some(a)) => a,
        Ok(None) => { app.status = "AC not found".into(); return; }
        Err(e) => { app.status = format!("get AC: {e}"); return; }
    };

    let tmp = format!("/tmp/coherence-ac-{}.md", ac.id);
    if std::fs::write(&tmp, &ac.intent).is_err() {
        app.status = "write failed".into();
        return;
    }

    let editor = env::var("EDITOR").or_else(|_| env::var("VISUAL")).unwrap_or_else(|_| "micro".to_string());
    let ok = Command::new(&editor).arg(&tmp).status().map(|s| s.success()).unwrap_or(false);

    if ok {
        let new_intent = std::fs::read_to_string(&tmp).unwrap_or_default();
        let mut updated = ac.clone();
        updated.intent = new_intent;
        match repo(app).put_acceptance_criterion(&updated) {
            Ok(()) => {
                app.status = "AC intent updated".into();
                if let Some(ref graph) = app.graph {
                    let mut g = graph.clone();
                    if let Some(a) = g.acceptance_criteria.iter_mut().find(|a| a.id == updated.id) {
                        a.intent = updated.intent.clone();
                    }
                    app.graph = Some(g);
                }
            }
            Err(e) => app.status = format!("update failed: {e}"),
        }
    } else {
        app.status = "Edit cancelled".into();
    }
    let _ = std::fs::remove_file(&tmp);
}

fn refresh_graph(app: &mut AppState) {
    let proj_path = app.projects[app.selected_project].0.clone();

    let mut repo = match DoltSpecRepository::new(proj_path.clone()) {
        Ok(r) => r,
        Err(e) => {
            app.status = format!("DB connect failed: {e}");
            return;
        }
    };

    match repo.load_spec_graph() {
        Ok(graph) => {
            app.project_dir = Some(proj_path.clone());
            app.repo = Some(Box::new(repo));
            app.graph = Some(graph);
            app.build_tree();
            app.status = format!("Loaded specs from {}", proj_path.display());
        }
        Err(e) => {
            app.status = format!("Load failed: {e}");
        }
    }
    app.update_preview();
}
