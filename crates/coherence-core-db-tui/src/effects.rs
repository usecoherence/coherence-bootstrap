use std::env;
use std::process::Command;

use coherence_core_db::ac_verify::{AcVerifyAcRunResult, VerifySpecRunResult};
use coherence_core_db::models::{AcceptanceCriterion, Spec, SpecLevel};

use crate::app::AppState;
use crate::edit::Draft;
use crate::repository::{DoltSpecRepository, SpecRepository};

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    PersistSpec(Spec),
    PersistAc(AcceptanceCriterion),
    OpenEditorForSpec(String),
    OpenEditorForAc(String),
    RefreshGraph,
    VerifyAc(String),
    VerifySpec(String),
    VerifyLevel(SpecLevel),
    VerifyAll,
}

pub fn execute_effects(app: &mut AppState, effects: Vec<Effect>) {
    for effect in effects {
        match effect {
            Effect::PersistSpec(spec) => persist_spec(app, spec),
            Effect::PersistAc(ac) => persist_ac(app, ac),
            Effect::OpenEditorForSpec(spec_id) => open_editor_for_spec(app, &spec_id),
            Effect::OpenEditorForAc(ac_id) => open_editor_for_ac(app, &ac_id),
            Effect::RefreshGraph => refresh_graph(app),
            Effect::VerifyAc(ac_id) => verify_ac(app, &ac_id),
            Effect::VerifySpec(spec_id) => verify_spec(app, &spec_id),
            Effect::VerifyLevel(level) => verify_level(app, level),
            Effect::VerifyAll => verify_all(app),
        }
    }
}

fn repo(app: &mut AppState) -> &mut dyn SpecRepository {
    app.repo
        .as_mut()
        .expect("SpecRepository not initialized")
        .as_mut()
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
        Ok(None) => {
            app.status = "spec not found".into();
            return;
        }
        Err(e) => {
            app.status = format!("get spec: {e}");
            return;
        }
    };

    let tmp = format!("/tmp/coherence-spec-{}.md", spec.id);
    let initial = match app.draft.as_ref() {
        Some(Draft::Spec {
            pending_description: Some(desc),
            ..
        }) => desc.clone(),
        _ => spec.description.clone(),
    };
    if std::fs::write(&tmp, &initial).is_err() {
        app.status = "write failed".into();
        return;
    }

    let editor = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| "micro".to_string());
    let ok = Command::new(&editor)
        .arg(&tmp)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        let new_desc = std::fs::read_to_string(&tmp).unwrap_or_default();
        match app.draft.as_mut() {
            Some(Draft::Spec {
                pending_description,
                ..
            }) => {
                *pending_description = Some(new_desc);
                app.status = "Description updated in draft".into();
            }
            _ => app.status = "No active draft".into(),
        }
    } else {
        app.status = "Edit cancelled".into();
    }
    let _ = std::fs::remove_file(&tmp);
}

fn open_editor_for_ac(app: &mut AppState, ac_id: &str) {
    let ac = match repo(app).get_acceptance_criterion(ac_id) {
        Ok(Some(a)) => a,
        Ok(None) => {
            app.status = "AC not found".into();
            return;
        }
        Err(e) => {
            app.status = format!("get AC: {e}");
            return;
        }
    };

    let tmp = format!("/tmp/coherence-ac-{}.md", ac.id);
    let initial = match app.draft.as_ref() {
        Some(Draft::Ac {
            pending_intent: Some(intent),
            ..
        }) => intent.clone(),
        _ => ac.intent.clone(),
    };
    if std::fs::write(&tmp, &initial).is_err() {
        app.status = "write failed".into();
        return;
    }

    let editor = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| "micro".to_string());
    let ok = Command::new(&editor)
        .arg(&tmp)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        let new_intent = std::fs::read_to_string(&tmp).unwrap_or_default();
        match app.draft.as_mut() {
            Some(Draft::Ac { pending_intent, .. }) => {
                *pending_intent = Some(new_intent);
                app.status = "Intent updated in draft".into();
            }
            _ => app.status = "No active draft".into(),
        }
    } else {
        app.status = "Edit cancelled".into();
    }
    let _ = std::fs::remove_file(&tmp);
}

fn refresh_graph(app: &mut AppState) {
    let proj_path = app.projects[app.selected_project].0.clone();
    let selected_env = app
        .envs
        .get(app.selected_env)
        .cloned()
        .unwrap_or_else(|| "dev".to_string());
    let previous_env = env::var("COHERENCE_ENV").ok();
    env::set_var("COHERENCE_ENV", &selected_env);

    let loaded = (|| {
        let mut repo = DoltSpecRepository::new(proj_path.clone())
            .map_err(|e| format!("DB connect failed: {e}"))?;
        let graph = repo
            .load_spec_graph()
            .map_err(|e| format!("Load failed: {e}"))?;
        Ok::<_, String>((repo, graph))
    })();

    match previous_env {
        Some(value) => env::set_var("COHERENCE_ENV", value),
        None => env::remove_var("COHERENCE_ENV"),
    }

    match loaded {
        Ok((mut repo, graph)) => {
            app.verification_statuses.clear();
            for ac in &graph.acceptance_criteria {
                if let Ok(Some(latest)) = repo.get_ac_verification_latest(&ac.id) {
                    app.verification_statuses
                        .insert(ac.id.clone(), latest.overall_status);
                }
            }
            app.project_dir = Some(proj_path.clone());
            app.repo = Some(Box::new(repo));
            app.graph = Some(graph);
            app.build_tree();
            app.status = format!("Loaded {selected_env} specs from {}", proj_path.display());
        }
        Err(e) => {
            app.status = e;
        }
    }
    app.update_preview();
}

fn verify_ac(app: &mut AppState, ac_id: &str) {
    match repo(app).verify_acceptance_criterion(ac_id) {
        Ok(result) => {
            app.verification_statuses
                .insert(result.ac_id.clone(), result.overall_status());
            app.status = verify_ac_summary(&result);
        }
        Err(e) => app.status = format!("verify AC failed: {e}"),
    }
}

fn verify_spec(app: &mut AppState, spec_id: &str) {
    match repo(app).verify_spec(spec_id) {
        Ok(result) => {
            update_statuses_from_report(app, &result);
            app.status = verify_spec_summary("Spec", &result);
        }
        Err(e) => app.status = format!("verify spec failed: {e}"),
    }
}

fn verify_level(app: &mut AppState, level: SpecLevel) {
    let Some(graph) = app.graph.clone() else {
        app.status = "No spec graph loaded".into();
        return;
    };
    let spec_ids: Vec<String> = graph
        .specs
        .iter()
        .filter(|spec| spec.level == level)
        .map(|spec| spec.id.clone())
        .collect();
    verify_specs(app, &format!("{} level", level.as_db_str()), &spec_ids);
}

fn verify_all(app: &mut AppState) {
    let Some(graph) = app.graph.clone() else {
        app.status = "No spec graph loaded".into();
        return;
    };
    let spec_ids: Vec<String> = graph.specs.iter().map(|spec| spec.id.clone()).collect();
    verify_specs(app, "All specs", &spec_ids);
}

fn verify_specs(app: &mut AppState, label: &str, spec_ids: &[String]) {
    if spec_ids.is_empty() {
        app.status = format!("{label}: no specs to verify");
        return;
    }

    let mut reports = Vec::with_capacity(spec_ids.len());
    for spec_id in spec_ids {
        match repo(app).verify_spec(spec_id) {
            Ok(report) => reports.push(report),
            Err(e) => {
                app.status = format!("verify {label} failed: {e}");
                return;
            }
        }
    }

    for report in &reports {
        update_statuses_from_report(app, report);
    }

    let acs: usize = reports.iter().map(|r| r.acceptance_criteria).sum();
    let passed: usize = reports.iter().map(|r| r.passed).sum();
    let failed: usize = reports.iter().map(|r| r.failed).sum();
    let skipped: usize = reports.iter().map(|r| r.skipped).sum();
    let no_verification: usize = reports.iter().map(|r| r.no_verification).sum();
    app.status = format!(
        "{label}: {acs} ACs, {passed} passed, {failed} failed, {skipped} skipped, {no_verification} no verification"
    );
}

fn update_statuses_from_report(app: &mut AppState, report: &VerifySpecRunResult) {
    for result in &report.ac_results {
        app.verification_statuses
            .insert(result.ac_id.clone(), result.overall_status());
    }
}

fn verify_ac_summary(result: &AcVerifyAcRunResult) -> String {
    format!("AC {}: {}", result.ac_id, result.overall_status_label())
}

fn verify_spec_summary(label: &str, result: &VerifySpecRunResult) -> String {
    format!(
        "{label} {}: {} ACs, {} passed, {} failed, {} skipped, {} no verification",
        result.spec_id,
        result.acceptance_criteria,
        result.passed,
        result.failed,
        result.skipped,
        result.no_verification
    )
}
