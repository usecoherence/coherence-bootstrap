use std::collections::HashMap;
use std::path::PathBuf;

use coherence_core_db::ac_verify::AcVerifyOverallStatus;

use crate::edit::Draft;
use crate::repository::SpecRepository;
use crate::tree;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    ProjectPicker,
    EnvPicker,
    Specs,
}

pub struct AppState {
    pub screen: Screen,
    pub focus_tree: bool,
    pub edit_mode: bool,
    pub draft: Option<Draft>,
    pub detail_scroll: u16,
    pub projects: Vec<(PathBuf, String)>,
    pub selected_project: usize,
    pub envs: Vec<String>,
    pub selected_env: usize,
    pub graph: Option<coherence_core_db::models::SpecGraph>,

    pub tree_items: Vec<tree::TreeItem>,
    pub selected_tree: usize,
    pub tree_scroll: usize,
    pub detail_spec_id: Option<String>,
    pub detail_ac_id: Option<String>,
    pub verification_statuses: HashMap<String, AcVerifyOverallStatus>,

    pub status: String,

    pub project_dir: Option<PathBuf>,
    pub repo: Option<Box<dyn SpecRepository>>,
}

impl AppState {
    pub fn new(projects: Vec<(PathBuf, String)>) -> Self {
        Self {
            screen: Screen::ProjectPicker,
            focus_tree: true,
            edit_mode: false,
            draft: None,
            detail_scroll: 0,
            projects,
            selected_project: 0,
            envs: vec!["dev".into(), "test".into(), "prod".into()],
            selected_env: 0,
            graph: None,
            tree_items: Vec::new(),
            selected_tree: 0,
            tree_scroll: 0,
            detail_spec_id: None,
            detail_ac_id: None,
            verification_statuses: HashMap::new(),
            status: "Select a project".into(),
            project_dir: None,
            repo: None,
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
        self.tree_scroll = 0;
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
        self.selected_tree = self
            .selected_tree
            .min(self.tree_items.len().saturating_sub(1));
        self.tree_scroll = self.tree_scroll.min(self.selected_tree);
    }

    pub fn ensure_tree_selection_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 || self.tree_items.is_empty() {
            self.tree_scroll = 0;
            return;
        }
        if self.selected_tree < self.tree_scroll {
            self.tree_scroll = self.selected_tree;
            return;
        }
        let bottom = self.tree_scroll + viewport_height;
        if self.selected_tree >= bottom {
            self.tree_scroll = self.selected_tree + 1 - viewport_height;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_tree_selection_visible_scrolls_down_to_selected_row() {
        let mut app = AppState::new(vec![]);
        app.tree_items = (0..20)
            .map(|i| tree::TreeItem {
                indent: 0,
                label: format!("item-{i}"),
                id: i.to_string(),
                is_spec: false,
                expanded: false,
                has_children: false,
                parent_spec_id: None,
            })
            .collect();
        app.selected_tree = 7;

        app.ensure_tree_selection_visible(5);

        assert_eq!(app.tree_scroll, 3);
    }

    #[test]
    fn ensure_tree_selection_visible_scrolls_up_to_selected_row() {
        let mut app = AppState::new(vec![]);
        app.tree_items = (0..20)
            .map(|i| tree::TreeItem {
                indent: 0,
                label: format!("item-{i}"),
                id: i.to_string(),
                is_spec: false,
                expanded: false,
                has_children: false,
                parent_spec_id: None,
            })
            .collect();
        app.tree_scroll = 10;
        app.selected_tree = 4;

        app.ensure_tree_selection_visible(5);

        assert_eq!(app.tree_scroll, 4);
    }
}
