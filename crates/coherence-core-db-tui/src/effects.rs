use std::env;
use std::process::Command;

use coherence_core_db::db::ConnectionConfig;
use coherence_core_db::models::{AcceptanceCriterion, Spec};
use coherence_core_db::spec_store;

use crate::app::AppState;

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

fn with_project_dir(app: &AppState) -> Option<impl FnOnce()> {
    let orig = env::current_dir().ok()?;
    let proj_path = app.projects[app.selected_project].0.clone();
    env::set_current_dir(&proj_path).ok()?;
    Some(move || { let _ = env::set_current_dir(&orig); })
}

fn db_conn() -> Option<mysql::Conn> {
    let config = ConnectionConfig::from_env().ok()?;
    let (conn, _) = coherence_core_db::db::connect(&config).ok()?;
    Some(conn)
}

fn persist_spec(app: &mut AppState, spec: Spec) {
    let restore = with_project_dir(app);
        let mut conn = match db_conn() {
        Some(c) => c,
        None => { if let Some(r) = restore { r(); } return; }
    };
    match spec_store::put_spec(&mut conn, &spec) {
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
    if let Some(r) = restore { r(); }
}

fn persist_ac(app: &mut AppState, ac: AcceptanceCriterion) {
    let restore = with_project_dir(app);
        let mut conn = match db_conn() {
        Some(c) => c,
        None => { if let Some(r) = restore { r(); } return; }
    };
    match spec_store::put_acceptance_criterion(&mut conn, &ac) {
        Ok(()) => {
            let field = match ac.review_mode.as_db_str() {
                "manual" | "automated" | "hybrid" => format!("Review → {}", ac.review_mode.as_db_str()),
                _ => format!("Risk → {}", ac.risk_level.as_db_str()),
            };
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
    if let Some(r) = restore { r(); }
}

fn open_editor_for_spec(app: &mut AppState, spec_id: &str) {
    let restore = with_project_dir(app);
        let mut conn = match db_conn() {
        Some(c) => c,
        None => { if let Some(r) = restore { r(); } return; }
    };

    let spec = match spec_store::get_spec(&mut conn, spec_id) {
        Ok(Some(s)) => s,
        _ => {
            app.status = "spec not found".into();
            if let Some(r) = restore { r(); }
            return;
        }
    };

    let tmp = format!("/tmp/coherence-spec-{}.md", spec.id);
    if std::fs::write(&tmp, &spec.description).is_err() {
        app.status = "write failed".into();
        if let Some(r) = restore { r(); }
        return;
    }

    let editor = env::var("EDITOR").or_else(|_| env::var("VISUAL")).unwrap_or_else(|_| "micro".to_string());
    let ok = Command::new(&editor).arg(&tmp).status().map(|s| s.success()).unwrap_or(false);

    if ok {
        let new_desc = std::fs::read_to_string(&tmp).unwrap_or_default();
        let mut updated = spec.clone();
        updated.description = new_desc;
        match spec_store::put_spec(&mut conn, &updated) {
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
    if let Some(r) = restore { r(); }
}

fn open_editor_for_ac(app: &mut AppState, ac_id: &str) {
    let restore = with_project_dir(app);
        let mut conn = match db_conn() {
        Some(c) => c,
        None => { if let Some(r) = restore { r(); } return; }
    };

    let ac = match spec_store::get_acceptance_criterion(&mut conn, ac_id) {
        Ok(Some(a)) => a,
        _ => {
            app.status = "AC not found".into();
            if let Some(r) = restore { r(); }
            return;
        }
    };

    let tmp = format!("/tmp/coherence-ac-{}.md", ac.id);
    if std::fs::write(&tmp, &ac.intent).is_err() {
        app.status = "write failed".into();
        if let Some(r) = restore { r(); }
        return;
    }

    let editor = env::var("EDITOR").or_else(|_| env::var("VISUAL")).unwrap_or_else(|_| "micro".to_string());
    let ok = Command::new(&editor).arg(&tmp).status().map(|s| s.success()).unwrap_or(false);

    if ok {
        let new_intent = std::fs::read_to_string(&tmp).unwrap_or_default();
        let mut updated = ac.clone();
        updated.intent = new_intent;
        match spec_store::put_acceptance_criterion(&mut conn, &updated) {
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
    if let Some(r) = restore { r(); }
}

fn refresh_graph(app: &mut AppState) {
    let restore = with_project_dir(app);
        let mut conn = match db_conn() {
        Some(c) => c,
        None => { if let Some(r) = restore { r(); } return; }
    };
    match spec_store::load_spec_graph(&mut conn) {
        Ok(graph) => {
            let proj_path = app.projects[app.selected_project].0.clone();
            app.graph = Some(graph);
            app.build_tree();
            app.status = format!("Loaded specs from {}", proj_path.display());
        }
        Err(e) => app.status = format!("Load failed: {e}"),
    }
    if let Some(r) = restore { r(); }
    app.update_preview();
}
