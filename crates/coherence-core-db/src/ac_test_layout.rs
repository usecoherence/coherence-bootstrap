//! Expected Rust acceptance-criterion test file paths and skeleton bodies from a [`SpecGraph`].
//!
//! # Path rule
//!
//! Each AC yields a path:
//!
//! `tests/ac_<ac.slug>.rs`
//!
//! Files are flat under `tests/ac_` (no subdirectories) so Cargo auto-discovers them.
//!
//! # Root and ancestry (MVP)
//!
//! Parent links come only from [`SpecRelation`] rows with `relation_kind == "depends_on"`,
//! interpreted as **child depends on parent**: `source_spec_id` is the child spec and
//! `target_spec_id` is its parent (toward the root). The spec hierarchy is still computed
//! (for future use) but the flat path uses `tests/ac_<ac.slug>.rs` regardless.
//!
//! For a spec with multiple `depends_on` parents (data error or future fan-in), the **lexicographically
//! smallest** `target_spec_id` is kept so the layout stays deterministic.
//!
//! Cycles in `depends_on` are truncated: the walk stops before a spec id repeats. Relations whose
//! kinds are not `depends_on` are ignored. Spec ids referenced on edges but missing from `specs`
//! end the walk early.
//!
//! # `test_command` stub
//!
//! [`ExpectedAcTestFile::test_command`] is populated with `cargo test -p coherence-core-db-bootstrap <rust_fn_name>` where
//! `<rust_fn_name>` is [`slug_to_rust_ident`] for the AC slug. This is a deterministic, human-oriented
//! hint for `verify-ac`-style runners; it is not guaranteed to match final crate test discovery.
//!
//! # Rust test function names
//!
//! [`slug_to_rust_ident`] lowercases, maps runs of hyphens and non-alphanumeric characters to
//! single underscores, trims underscores, prefixes `validates_` when the body does not already
//! start with `validates_`, and uses `validates_ac_test` for an empty slug after sanitization.

use std::collections::{HashMap, HashSet};

use crate::models::{AcceptanceCriterion, Spec, SpecGraph, SpecRelation};

/// One AC test file that tooling may materialize or check on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedAcTestFile {
    pub ac_id: String,
    pub file_path: String,
    pub test_command: String,
    pub content: String,
}

/// Map an AC slug into a conventional `#[test]` fn name (`validates_…` + `snake_case` body).
#[must_use]
pub fn slug_to_rust_ident(slug: &str) -> String {
    let mut body = String::new();
    let lowered = slug.to_ascii_lowercase();
    let mut prev_sep = true;
    for ch in lowered.chars() {
        let is_alnum = ch.is_ascii_alphanumeric();
        if is_alnum {
            body.push(ch);
            prev_sep = false;
        } else if ch == '-' || ch == '_' {
            if !prev_sep && !body.is_empty() {
                body.push('_');
            }
            prev_sep = true;
        } else if !prev_sep && !body.is_empty() {
            body.push('_');
            prev_sep = true;
        }
    }
    while body.ends_with('_') {
        body.pop();
    }
    if body.is_empty() {
        return "validates_ac_test".to_string();
    }
    if body.starts_with("validates_") {
        body
    } else {
        format!("validates_{body}")
    }
}

/// Deterministic list of expected per-AC Rust test files (sorted by `ac_id`).
#[must_use]
pub fn expected_rust_ac_test_files(graph: &SpecGraph) -> Vec<ExpectedAcTestFile> {
    let spec_by_id: HashMap<&str, &Spec> = graph.specs.iter().map(|s| (s.id.as_str(), s)).collect();
    let parent_map = depends_on_parent_map(&graph.spec_relations);

    let mut acs: Vec<&AcceptanceCriterion> = graph
        .acceptance_criteria
        .iter()
        .filter(|ac| spec_by_id.contains_key(ac.spec_id.as_str()))
        .collect();
    acs.sort_by(|a, b| a.id.cmp(&b.id));

    let mut out = Vec::with_capacity(acs.len());
    for ac in acs {
        let _path_segments = spec_slug_path(ac.spec_id.as_str(), &spec_by_id, &parent_map);
        let file_path = format!("tests/ac_{}.rs", ac.slug);

        let ident = slug_to_rust_ident(&ac.slug);
        let test_command = format!("cargo test -p coherence-core-db-bootstrap {ident}");
        let content = skeleton_rust_content(&ac.id, &ident);

        out.push(ExpectedAcTestFile {
            ac_id: ac.id.clone(),
            file_path,
            test_command,
            content,
        });
    }
    out
}

fn depends_on_parent_map(relations: &[SpecRelation]) -> HashMap<String, String> {
    let mut best: HashMap<String, String> = HashMap::new();
    for rel in relations {
        if rel.relation_kind != "depends_on" {
            continue;
        }
        best.entry(rel.source_spec_id.clone())
            .and_modify(|cur| {
                if rel.target_spec_id < *cur {
                    cur.clone_from(&rel.target_spec_id);
                }
            })
            .or_insert_with(|| rel.target_spec_id.clone());
    }
    best
}

fn spec_slug_path(
    leaf_spec_id: &str,
    spec_by_id: &HashMap<&str, &Spec>,
    parent_map: &HashMap<String, String>,
) -> Vec<String> {
    let mut chain_ids: Vec<String> = Vec::new();
    let mut visited = HashSet::<String>::new();
    let mut cur = leaf_spec_id.to_string();

    loop {
        chain_ids.push(cur.clone());
        visited.insert(cur.clone());
        let Some(parent_id) = parent_map.get(&cur) else {
            break;
        };
        if !spec_by_id.contains_key(parent_id.as_str()) {
            break;
        }
        if visited.contains(parent_id) {
            break;
        }
        cur = parent_id.clone();
    }

    chain_ids.reverse();
    chain_ids
        .into_iter()
        .filter_map(|id| spec_by_id.get(id.as_str()).map(|s| s.slug.clone()))
        .collect()
}

fn skeleton_rust_content(ac_id: &str, ident: &str) -> String {
    format!(
        "//! AC: {ac_id}\n//! Generated by coherence-core-db MVP AC test layout.\n\n#[test]\nfn {ident}() {{\n    todo!(\"Implement {ac_id}\");\n}}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Spec, SpecGraph, SpecRelation};

    #[test]
    fn slug_to_rust_ident_hyphen_example() {
        assert_eq!(
            slug_to_rust_ident("rejects-malformed-url"),
            "validates_rejects_malformed_url"
        );
    }

    #[test]
    fn slug_to_rust_ident_no_double_validates_prefix() {
        assert_eq!(slug_to_rust_ident("validates-already"), "validates_already");
    }

    #[test]
    fn slug_to_rust_ident_sanitizes_weird_chars() {
        assert_eq!(
            slug_to_rust_ident("weird!!slug--1"),
            "validates_weird_slug_1"
        );
    }

    #[test]
    fn file_path_uses_ac_slug_not_title() {
        let mut ac =
            crate::models::AcceptanceCriterion::new("AC-1", "SPEC-ROOT", "Human Readable Title");
        ac.slug = "machine-slug".to_string();

        let g = SpecGraph::new(vec![Spec::new("SPEC-ROOT", "Root")], vec![ac], vec![]);

        let files = expected_rust_ac_test_files(&g);
        assert_eq!(files.len(), 1);
        assert!(
            files[0].file_path.ends_with("tests/ac_machine-slug.rs"),
            "got {}",
            files[0].file_path
        );
        assert!(!files[0].file_path.contains("human"));
    }

    #[test]
    fn multi_segment_path_walks_depends_on_chain() {
        let mut root = Spec::new("SPEC-COREDB", "Coherence core DB");
        root.slug = "coredb".into();
        let mut child = Spec::new("SPEC-CODE-LINKS", "Code links");
        child.slug = "code-links".into();

        let rel = SpecRelation::new("REL-1", "SPEC-CODE-LINKS", "SPEC-COREDB", "depends_on", "");

        let mut ac = crate::models::AcceptanceCriterion::new("AC-1", "SPEC-CODE-LINKS", "t");
        ac.slug = "rejects-malformed-url".into();

        let g = SpecGraph::new(vec![root, child], vec![ac], vec![rel]);

        let files = expected_rust_ac_test_files(&g);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_path, "tests/ac_rejects-malformed-url.rs");
        assert_eq!(
            files[0].test_command,
            "cargo test -p coherence-core-db-bootstrap validates_rejects_malformed_url"
        );
        assert!(files[0]
            .content
            .contains("fn validates_rejects_malformed_url()"));
    }

    #[test]
    fn ignores_non_depends_on_relations_for_path() {
        let a = Spec::new("SPEC-A", "A");
        let b = Spec::new("SPEC-B", "B");
        let rel = SpecRelation::new("REL-1", "SPEC-B", "SPEC-A", "see_also", "");
        let ac = crate::models::AcceptanceCriterion::new("AC-1", "SPEC-B", "t");
        let g = SpecGraph::new(vec![a, b], vec![ac], vec![rel]);
        let files = expected_rust_ac_test_files(&g);
        assert_eq!(files[0].file_path, "tests/ac_ac-1.rs");
    }
}
