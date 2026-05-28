use std::env;
use std::path::PathBuf;
use std::process::Command;

use coherence_core_db::db::ConnectionConfig;
use coherence_core_db::project_manifest;
use coherence_core_db::spec_store;

use crate::tree;

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    ProjectPicker,
    EnvPicker,
    Specs,
}

#[derive(Clone)]
pub struct AppState {
    pub screen: Screen,
    pub focus_tree: bool,
    pub edit_mode: bool,
    pub detail_scroll: u16,
    pub projects: Vec<(PathBuf, String)>,
    pub selected_project: usize,
    pub envs: Vec<String>,
    pub selected_env: usize,
    pub graph: Option<coherence_core_db::models::SpecGraph>,

    pub tree_items: Vec<tree::TreeItem>,
    pub selected_tree: usize,
    pub detail_spec_id: Option<String>,
    pub detail_ac_id: Option<String>,

    pub status: String,
}

impl AppState {
    pub fn new(projects: Vec<(PathBuf, String)>) -> Self {
        Self {
            screen: Screen::ProjectPicker,
            focus_tree: true,
            edit_mode: false,
            detail_scroll: 0,
            projects,
            selected_project: 0,
            envs: vec!["dev".into(), "test".into(), "prod".into()],
            selected_env: 0,
            graph: None,
            tree_items: Vec::new(),
            selected_tree: 0,
            detail_spec_id: None,
            detail_ac_id: None,
            status: "Select a project".into(),
        }
    }

    pub fn edit_content(&mut self) {
        let proj_path = self.projects[self.selected_project].0.clone();
        let orig = env::current_dir().ok();
        let _ = env::set_current_dir(&proj_path);

        let config = match ConnectionConfig::from_env() {
            Ok(c) => c,
            Err(_) => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; }
        };
        let (mut conn, _) = match coherence_core_db::db::connect(&config) {
            Ok(v) => v,
            Err(_) => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; }
        };

        let (spec_id, ac_id) = (self.detail_spec_id.clone(), self.detail_ac_id.clone());

        if let Some(sid) = spec_id {
            let spec = match spec_store::get_spec(&mut conn, &sid) {
                Ok(Some(s)) => s,
                _ => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; }
            };

            let tmp = format!("/tmp/coherence-spec-{}.md", spec.id);
            if std::fs::write(&tmp, &spec.description).is_err() {
                self.status = "write failed".into();
                if let Some(p) = orig { let _ = env::set_current_dir(p); } return;
            }

            let editor = env::var("EDITOR").or_else(|_| env::var("VISUAL")).unwrap_or_else(|_| "micro".to_string());
            let ok = Command::new(&editor).arg(&tmp).status().map(|s| s.success()).unwrap_or(false);

            if ok {
                let new_desc = std::fs::read_to_string(&tmp).unwrap_or_default();
                let mut updated = spec.clone();
                updated.description = new_desc;
                match spec_store::put_spec(&mut conn, &updated) {
                    Ok(()) => {
                        self.status = "Spec description updated".into();
                        if let Some(ref graph) = self.graph {
                            let mut g = graph.clone();
                            if let Some(s) = g.specs.iter_mut().find(|s| s.id == updated.id) {
                                s.description = updated.description.clone();
                            }
                            self.graph = Some(g);
                        }
                    }
                    Err(e) => self.status = format!("update failed: {e}"),
                }
            } else {
                self.status = "Edit cancelled".into();
            }
            let _ = std::fs::remove_file(&tmp);
        } else if let Some(aid) = ac_id {
            let ac = match spec_store::get_acceptance_criterion(&mut conn, &aid) {
                Ok(Some(a)) => a,
                _ => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; }
            };

            let tmp = format!("/tmp/coherence-ac-{}.md", ac.id);
            if std::fs::write(&tmp, &ac.intent).is_err() {
                self.status = "write failed".into();
                if let Some(p) = orig { let _ = env::set_current_dir(p); } return;
            }

            let editor = env::var("EDITOR").or_else(|_| env::var("VISUAL")).unwrap_or_else(|_| "micro".to_string());
            let ok = Command::new(&editor).arg(&tmp).status().map(|s| s.success()).unwrap_or(false);

            if ok {
                let new_intent = std::fs::read_to_string(&tmp).unwrap_or_default();
                let mut updated = ac.clone();
                updated.intent = new_intent;
                match spec_store::put_acceptance_criterion(&mut conn, &updated) {
                    Ok(()) => {
                        self.status = "AC intent updated".into();
                        if let Some(ref graph) = self.graph {
                            let mut g = graph.clone();
                            if let Some(a) = g.acceptance_criteria.iter_mut().find(|a| a.id == updated.id) {
                                a.intent = updated.intent.clone();
                            }
                            self.graph = Some(g);
                        }
                    }
                    Err(e) => self.status = format!("update failed: {e}"),
                }
            } else {
                self.status = "Edit cancelled".into();
            }
            let _ = std::fs::remove_file(&tmp);
        }

        if let Some(p) = orig { let _ = env::set_current_dir(p); }
    }

    pub fn cycle_status(&mut self) {
        let sid = match self.detail_spec_id.clone() { Some(id) => id, None => { self.status = "No spec selected".into(); return; } };
        let proj_path = self.projects[self.selected_project].0.clone();
        let orig = env::current_dir().ok();
        let _ = env::set_current_dir(&proj_path);
        let config = ConnectionConfig::from_env().ok();
        let conn = config.and_then(|c| coherence_core_db::db::connect(&c).ok());
        let (mut conn, _) = match conn { Some(v) => v, None => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };

        let spec = match spec_store::get_spec(&mut conn, &sid) { Ok(Some(s)) => s, _ => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };
        let next = match spec.status {
            coherence_core_db::models::SpecStatus::Draft => coherence_core_db::models::SpecStatus::Active,
            coherence_core_db::models::SpecStatus::Active => coherence_core_db::models::SpecStatus::Deprecated,
            coherence_core_db::models::SpecStatus::Deprecated => coherence_core_db::models::SpecStatus::Archived,
            coherence_core_db::models::SpecStatus::Archived => coherence_core_db::models::SpecStatus::Draft,
        };
        let mut updated = spec.clone();
        updated.status = next;
        if spec_store::put_spec(&mut conn, &updated).is_ok() {
            self.status = format!("Status → {}", next.as_db_str());
            if let Some(ref graph) = self.graph {
                let mut g = graph.clone();
                if let Some(s) = g.specs.iter_mut().find(|s| s.id == updated.id) { s.status = next; }
                self.graph = Some(g);
            }
        }
        if let Some(p) = orig { let _ = env::set_current_dir(p); }
    }

    pub fn cycle_level(&mut self) {
        let sid = match self.detail_spec_id.clone() { Some(id) => id, None => { self.status = "No spec selected".into(); return; } };
        let proj_path = self.projects[self.selected_project].0.clone();
        let orig = env::current_dir().ok();
        let _ = env::set_current_dir(&proj_path);
        let config = ConnectionConfig::from_env().ok();
        let conn = config.and_then(|c| coherence_core_db::db::connect(&c).ok());
        let (mut conn, _) = match conn { Some(v) => v, None => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };

        let spec = match spec_store::get_spec(&mut conn, &sid) { Ok(Some(s)) => s, _ => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };
        let next = match spec.level {
            coherence_core_db::models::SpecLevel::Product => coherence_core_db::models::SpecLevel::System,
            coherence_core_db::models::SpecLevel::System => coherence_core_db::models::SpecLevel::Module,
            coherence_core_db::models::SpecLevel::Module => coherence_core_db::models::SpecLevel::Product,
        };
        let mut updated = spec.clone();
        updated.level = next;
        if spec_store::put_spec(&mut conn, &updated).is_ok() {
            self.status = format!("Level → {}", next.as_db_str());
            if let Some(ref graph) = self.graph {
                let mut g = graph.clone();
                if let Some(s) = g.specs.iter_mut().find(|s| s.id == updated.id) { s.level = next; }
                self.graph = Some(g);
            }
        }
        if let Some(p) = orig { let _ = env::set_current_dir(p); }
    }

    pub fn cycle_review_mode(&mut self) {
        let aid = match self.detail_ac_id.clone() { Some(id) => id, None => { self.status = "No AC selected".into(); return; } };
        let proj_path = self.projects[self.selected_project].0.clone();
        let orig = env::current_dir().ok();
        let _ = env::set_current_dir(&proj_path);
        let config = ConnectionConfig::from_env().ok();
        let conn = config.and_then(|c| coherence_core_db::db::connect(&c).ok());
        let (mut conn, _) = match conn { Some(v) => v, None => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };

        let ac = match spec_store::get_acceptance_criterion(&mut conn, &aid) { Ok(Some(a)) => a, _ => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };
        let next = match ac.review_mode {
            coherence_core_db::models::ReviewMode::Manual => coherence_core_db::models::ReviewMode::Automated,
            coherence_core_db::models::ReviewMode::Automated => coherence_core_db::models::ReviewMode::Hybrid,
            coherence_core_db::models::ReviewMode::Hybrid => coherence_core_db::models::ReviewMode::Manual,
        };
        let mut updated = ac.clone();
        updated.review_mode = next;
        if spec_store::put_acceptance_criterion(&mut conn, &updated).is_ok() {
            self.status = format!("Review → {}", next.as_db_str());
            if let Some(ref graph) = self.graph {
                let mut g = graph.clone();
                if let Some(a) = g.acceptance_criteria.iter_mut().find(|a| a.id == updated.id) { a.review_mode = next; }
                self.graph = Some(g);
            }
        }
        if let Some(p) = orig { let _ = env::set_current_dir(p); }
    }

    pub fn cycle_risk_level(&mut self) {
        let aid = match self.detail_ac_id.clone() { Some(id) => id, None => { self.status = "No AC selected".into(); return; } };
        let proj_path = self.projects[self.selected_project].0.clone();
        let orig = env::current_dir().ok();
        let _ = env::set_current_dir(&proj_path);
        let config = ConnectionConfig::from_env().ok();
        let conn = config.and_then(|c| coherence_core_db::db::connect(&c).ok());
        let (mut conn, _) = match conn { Some(v) => v, None => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };

        let ac = match spec_store::get_acceptance_criterion(&mut conn, &aid) { Ok(Some(a)) => a, _ => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };
        let next = match ac.risk_level {
            coherence_core_db::models::RiskLevel::Low => coherence_core_db::models::RiskLevel::Medium,
            coherence_core_db::models::RiskLevel::Medium => coherence_core_db::models::RiskLevel::High,
            coherence_core_db::models::RiskLevel::High => coherence_core_db::models::RiskLevel::Critical,
            coherence_core_db::models::RiskLevel::Critical => coherence_core_db::models::RiskLevel::Low,
        };
        let mut updated = ac.clone();
        updated.risk_level = next;
        if spec_store::put_acceptance_criterion(&mut conn, &updated).is_ok() {
            self.status = format!("Risk → {}", next.as_db_str());
            if let Some(ref graph) = self.graph {
                let mut g = graph.clone();
                if let Some(a) = g.acceptance_criteria.iter_mut().find(|a| a.id == updated.id) { a.risk_level = next; }
                self.graph = Some(g);
            }
        }
        if let Some(p) = orig { let _ = env::set_current_dir(p); }
    }

    pub fn load_graph(&mut self) {
        let proj_path = self.projects[self.selected_project].0.clone();
        let env_str = self.envs[self.selected_env].clone();
        let _coherence_env = match env_str.as_str() {
            "test" => project_manifest::CoherenceEnv::Test,
            "prod" => project_manifest::CoherenceEnv::Prod,
            _ => project_manifest::CoherenceEnv::Dev,
        };

        let orig = env::current_dir().ok();
        let _ = env::set_current_dir(&proj_path);

        let config = match ConnectionConfig::from_env() {
            Ok(c) => c,
            Err(e) => {
                self.status = format!("DB connect failed: {e}");
                if let Some(p) = orig { let _ = env::set_current_dir(p); }
                return;
            }
        };

        let (mut conn, _) = match coherence_core_db::db::connect(&config) {
            Ok(v) => v,
            Err(e) => {
                self.status = format!("DB connect failed: {e}");
                if let Some(p) = orig { let _ = env::set_current_dir(p); }
                return;
            }
        };

        match spec_store::load_spec_graph(&mut conn) {
            Ok(graph) => {
                self.graph = Some(graph);
                self.build_tree();
                self.status = format!("Loaded specs from {}", proj_path.display());
            }
            Err(e) => { self.status = format!("Load failed: {e}"); }
        }

        if let Some(p) = orig { let _ = env::set_current_dir(p); }
    }

    pub fn update_preview(&mut self) {
        let (sid, aid) = tree::update_preview(&self.tree_items, self.selected_tree);
        self.detail_spec_id = sid;
        self.detail_ac_id = aid;
        self.detail_scroll = 0;
    }

    pub fn build_tree(&mut self) {
        self.selected_tree = 0;
        let Some(ref graph) = self.graph.clone() else {
            self.tree_items.clear();
            return;
        };
        tree::build_tree(&mut self.tree_items, &graph);
    }

    pub fn toggle_expand(&mut self) {
        let Some(ref graph) = self.graph.clone() else {
            return;
        };
        tree::toggle_expand(&mut self.tree_items, self.selected_tree, &graph);
    }
}
