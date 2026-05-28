use std::path::PathBuf;

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
