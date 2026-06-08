use std::collections::HashMap;

use coherence_core_db::models::{SpecGraph, SpecLevel};

#[derive(Clone)]
pub struct TreeItem {
    pub indent: usize,
    pub label: String,
    pub id: String,
    pub is_spec: bool,
    pub expanded: bool,
    pub has_children: bool,
    pub parent_spec_id: Option<String>,
}

pub fn build_tree(items: &mut Vec<TreeItem>, graph: &SpecGraph) {
    items.clear();
    let mut has_product = false;
    let mut has_system = false;
    let mut has_module = false;
    let mut has_component = false;
    let mut has_foundation = false;

    for spec in &graph.specs {
        match spec.level {
            SpecLevel::Product => has_product = true,
            SpecLevel::System => has_system = true,
            SpecLevel::Module => has_module = true,
            SpecLevel::Component => has_component = true,
            SpecLevel::Foundation => has_foundation = true,
        }
    }

    for (level_name, has) in [
        ("Product", has_product),
        ("System", has_system),
        ("Module", has_module),
        ("Component", has_component),
        ("Foundation", has_foundation),
    ] {
        if !has {
            continue;
        }
        items.push(TreeItem {
            indent: 0,
            label: level_name.to_string(),
            id: String::new(),
            is_spec: false,
            expanded: false,
            has_children: true,
            parent_spec_id: None,
        });
    }
}

pub fn toggle_expand(items: &mut Vec<TreeItem>, idx: usize, graph: &SpecGraph) {
    if idx >= items.len() {
        return;
    }
    let has_children = items[idx].has_children;
    if !has_children {
        return;
    }

    let is_expanded = items[idx].expanded;
    let indent = items[idx].indent;

    if is_expanded {
        let mut to_remove = Vec::new();
        for (i, ti) in items.iter().enumerate().skip(idx + 1) {
            if ti.indent <= indent {
                break;
            }
            to_remove.push(i);
        }
        for i in to_remove.into_iter().rev() {
            items.remove(i);
        }
        items[idx].expanded = false;
    } else {
        let insert_at = idx + 1;

        match indent {
            0 => {
                let level_name = items[idx].label.clone();
                let Some(target_level) = level_from_label(&level_name) else {
                    return;
                };
                let acs_by_spec: HashMap<&str, usize> =
                    graph
                        .acceptance_criteria
                        .iter()
                        .fold(HashMap::new(), |mut acc, ac| {
                            *acc.entry(ac.spec_id.as_str()).or_insert(0) += 1;
                            acc
                        });

                let mut pos = insert_at;
                for spec in &graph.specs {
                    if spec.level != target_level {
                        continue;
                    }
                    let ac_count = acs_by_spec.get(spec.id.as_str()).copied().unwrap_or(0);
                    let label = format!(
                        "{}  {}",
                        spec.slug,
                        if ac_count > 0 {
                            format!("({})", ac_count)
                        } else {
                            String::new()
                        },
                    );
                    items.insert(
                        pos,
                        TreeItem {
                            indent: 1,
                            label,
                            id: spec.id.clone(),
                            is_spec: true,
                            expanded: false,
                            has_children: ac_count > 0,
                            parent_spec_id: None,
                        },
                    );
                    pos += 1;
                }
            }
            1 => {
                let spec_id = items[idx].id.clone();
                let mut pos = insert_at;
                for ac in &graph.acceptance_criteria {
                    if ac.spec_id != spec_id {
                        continue;
                    }
                    let label = format!("  {}", ac.slug);
                    items.insert(
                        pos,
                        TreeItem {
                            indent: 2,
                            label,
                            id: ac.id.clone(),
                            is_spec: false,
                            expanded: false,
                            has_children: false,
                            parent_spec_id: Some(ac.spec_id.clone()),
                        },
                    );
                    pos += 1;
                }
            }
            _ => {}
        }
        items[idx].expanded = true;
    }
}

fn level_from_label(label: &str) -> Option<SpecLevel> {
    match label {
        "Product" => Some(SpecLevel::Product),
        "System" => Some(SpecLevel::System),
        "Module" => Some(SpecLevel::Module),
        "Component" => Some(SpecLevel::Component),
        "Foundation" => Some(SpecLevel::Foundation),
        _ => None,
    }
}

pub fn update_preview(items: &[TreeItem], idx: usize) -> (Option<String>, Option<String>) {
    if idx >= items.len() {
        return (None, None);
    }
    let item = &items[idx];
    if item.is_spec {
        (Some(item.id.clone()), None)
    } else if item.parent_spec_id.is_some() {
        (None, Some(item.id.clone()))
    } else {
        (None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coherence_core_db::models::{AcceptanceCriterion, Spec};

    fn make_graph(specs: Vec<Spec>, acs: Vec<AcceptanceCriterion>) -> SpecGraph {
        SpecGraph::new(specs, acs, vec![])
    }

    #[test]
    fn build_tree_empty_graph_yields_empty_items() {
        let graph = make_graph(vec![], vec![]);
        let mut items = Vec::new();
        build_tree(&mut items, &graph);
        assert!(items.is_empty());
    }

    #[test]
    fn build_tree_creates_level_headers() {
        let s1 = Spec::new("S1", "spec one");
        let graph = make_graph(vec![s1], vec![]);
        let mut items = Vec::new();
        build_tree(&mut items, &graph);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "Module");
        assert!(items[0].has_children);
        assert!(!items[0].expanded);
    }

    #[test]
    fn toggle_expand_expands_specs_under_level() {
        let s1 = Spec::new("S1", "spec one");
        let graph = make_graph(vec![s1.clone()], vec![]);
        let mut items = Vec::new();
        build_tree(&mut items, &graph);
        toggle_expand(&mut items, 0, &graph);
        assert!(items[0].expanded);
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].label, "s1  ");
        assert!(items[1].is_spec);
    }

    #[test]
    fn toggle_expand_expands_component_and_foundation_specs() {
        let mut component = Spec::new("S1", "component spec");
        component.level = SpecLevel::Component;
        let mut foundation = Spec::new("S2", "foundation spec");
        foundation.level = SpecLevel::Foundation;
        let graph = make_graph(vec![component, foundation], vec![]);
        let mut items = Vec::new();
        build_tree(&mut items, &graph);

        assert_eq!(items[0].label, "Component");
        assert_eq!(items[1].label, "Foundation");

        toggle_expand(&mut items, 0, &graph);
        assert_eq!(items[1].label, "s1  ");

        toggle_expand(&mut items, 2, &graph);
        assert_eq!(items[3].label, "s2  ");
    }

    #[test]
    fn toggle_expand_collapses_specs() {
        let s1 = Spec::new("S1", "spec one");
        let graph = make_graph(vec![s1.clone()], vec![]);
        let mut items = Vec::new();
        build_tree(&mut items, &graph);
        toggle_expand(&mut items, 0, &graph);
        assert_eq!(items.len(), 2);
        toggle_expand(&mut items, 0, &graph);
        assert_eq!(items.len(), 1);
        assert!(!items[0].expanded);
    }

    #[test]
    fn toggle_expand_expands_acs_under_spec() {
        let s1 = Spec::new("S1", "spec one");
        let ac1 = AcceptanceCriterion::new("AC1", "S1", "ac one");
        let graph = make_graph(vec![s1], vec![ac1]);
        let mut items = Vec::new();
        build_tree(&mut items, &graph);
        toggle_expand(&mut items, 0, &graph);
        assert_eq!(items.len(), 2);
        toggle_expand(&mut items, 1, &graph);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn update_preview_returns_spec_id_for_spec() {
        let items = vec![
            TreeItem {
                indent: 0,
                label: "Module".into(),
                id: "".into(),
                is_spec: false,
                expanded: false,
                has_children: true,
                parent_spec_id: None,
            },
            TreeItem {
                indent: 1,
                label: "S1".into(),
                id: "S1".into(),
                is_spec: true,
                expanded: false,
                has_children: false,
                parent_spec_id: None,
            },
        ];
        let (sid, aid) = update_preview(&items, 1);
        assert_eq!(sid, Some("S1".into()));
        assert_eq!(aid, None);
    }

    #[test]
    fn update_preview_returns_ac_id_for_ac() {
        let items = vec![
            TreeItem {
                indent: 0,
                label: "Module".into(),
                id: "".into(),
                is_spec: false,
                expanded: false,
                has_children: true,
                parent_spec_id: None,
            },
            TreeItem {
                indent: 1,
                label: "S1".into(),
                id: "S1".into(),
                is_spec: true,
                expanded: false,
                has_children: false,
                parent_spec_id: None,
            },
            TreeItem {
                indent: 2,
                label: "ac1".into(),
                id: "AC1".into(),
                is_spec: false,
                expanded: false,
                has_children: false,
                parent_spec_id: Some("S1".into()),
            },
        ];
        let (sid, aid) = update_preview(&items, 2);
        assert_eq!(sid, None);
        assert_eq!(aid, Some("AC1".into()));
    }
}
