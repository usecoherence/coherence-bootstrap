//! Persistence for the **spec** module: specs, acceptance criteria, concerns, relations.
//!
//! Schema source: `sql/modules/spec/migrations/`. Ownership and boundary with **codeintel** are
//! described in `AGENTS.md` (“M1 module ownership”).
use mysql::prelude::Queryable;
use mysql::{params, Conn};

use crate::models::{
    AcceptanceCriterion, ConcernKind, ReviewMode, RiskLevel, Spec, SpecGraph, SpecLevel,
    SpecRelation, SpecStatus,
};

type SpecRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);
type AcceptanceCriterionRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

pub fn put_spec(conn: &mut Conn, spec: &Spec) -> Result<(), String> {
    conn.exec_drop(
        r"INSERT INTO specs (
            id,
            slug,
            title,
            description,
            level,
            status,
            created_at,
            updated_at
          ) VALUES (
            :id,
            :slug,
            :title,
            :description,
            :level,
            :status,
            :created_at,
            :updated_at
          )
          ON DUPLICATE KEY UPDATE
            slug = VALUES(slug),
            title = VALUES(title),
            description = VALUES(description),
            level = VALUES(level),
            status = VALUES(status),
            updated_at = VALUES(updated_at)",
        params! {
            "id" => spec.id.as_str(),
            "slug" => spec.slug.as_str(),
            "title" => spec.title.as_str(),
            "description" => spec.description.as_str(),
            "level" => spec.level.as_db_str(),
            "status" => spec.status.as_db_str(),
            "created_at" => spec.created_at.as_str(),
            "updated_at" => spec.updated_at.as_str(),
        },
    )
    .map_err(|err| format!("failed to put spec {}: {err}", spec.id))
}

pub fn get_spec(conn: &mut Conn, spec_id: &str) -> Result<Option<Spec>, String> {
    let row: Option<SpecRow> = conn
        .exec_first(
            r"SELECT id, slug, title, description, level, status, created_at, updated_at
              FROM specs
              WHERE id = :id",
            params! {
                "id" => spec_id,
            },
        )
        .map_err(|err| format!("failed to get spec {spec_id}: {err}"))?;
    row.map(spec_from_row).transpose()
}

pub fn list_specs(conn: &mut Conn) -> Result<Vec<Spec>, String> {
    let rows: Vec<SpecRow> = conn
        .query(
            r"SELECT id, slug, title, description, level, status, created_at, updated_at
              FROM specs
              ORDER BY id",
        )
        .map_err(|err| format!("failed to list specs: {err}"))?;
    rows.into_iter().map(spec_from_row).collect()
}

/// Loads every row from `specs`, `acceptance_criteria`, and `spec_relations` (three full-table
/// scans). See [`SpecGraph`] for MVP semantics (no integrity checks; AC concerns not populated).
#[allow(dead_code)] // Public bulk loader for embedders (COREDB-xah).
pub fn load_spec_graph(conn: &mut Conn) -> Result<SpecGraph, String> {
    let specs = list_specs(conn)?;

    let ac_rows: Vec<AcceptanceCriterionRow> = conn
        .query(
            r"SELECT id, spec_id, slug, title, intent, review_mode, risk_level, created_at, updated_at
              FROM acceptance_criteria
              ORDER BY id",
        )
        .map_err(|err| format!("failed to list acceptance criteria for spec graph: {err}"))?;

    let mut acceptance_criteria = Vec::with_capacity(ac_rows.len());
    for row in ac_rows {
        acceptance_criteria.push(acceptance_criterion_from_row(row)?);
    }

    let relation_rows: Vec<(String, String, String, String, String)> = conn
        .query(
            r"SELECT id, source_spec_id, target_spec_id, relation_kind, note
              FROM spec_relations
              ORDER BY id",
        )
        .map_err(|err| format!("failed to list spec relations for spec graph: {err}"))?;

    let spec_relations = relation_rows
        .into_iter()
        .map(
            |(id, source_spec_id, target_spec_id, relation_kind, note)| SpecRelation {
                id,
                source_spec_id,
                target_spec_id,
                relation_kind,
                note,
            },
        )
        .collect();

    Ok(SpecGraph::new(specs, acceptance_criteria, spec_relations))
}

pub fn put_acceptance_criterion(conn: &mut Conn, ac: &AcceptanceCriterion) -> Result<(), String> {
    reject_empty_slug(ac)?;
    reject_duplicate_slug_in_spec(conn, ac)?;

    conn.exec_drop(
        r"INSERT INTO acceptance_criteria (
            id,
            spec_id,
            slug,
            title,
            intent,
            review_mode,
            risk_level,
            created_at,
            updated_at
          ) VALUES (
            :id,
            :spec_id,
            :slug,
            :title,
            :intent,
            :review_mode,
            :risk_level,
            :created_at,
            :updated_at
          )
          ON DUPLICATE KEY UPDATE
            spec_id = VALUES(spec_id),
            slug = VALUES(slug),
            title = VALUES(title),
            intent = VALUES(intent),
            review_mode = VALUES(review_mode),
            risk_level = VALUES(risk_level),
            updated_at = VALUES(updated_at)",
        params! {
            "id" => ac.id.as_str(),
            "spec_id" => ac.spec_id.as_str(),
            "slug" => ac.slug.as_str(),
            "title" => ac.title.as_str(),
            "intent" => ac.intent.as_str(),
            "review_mode" => ac.review_mode.as_db_str(),
            "risk_level" => ac.risk_level.as_db_str(),
            "created_at" => ac.created_at.as_str(),
            "updated_at" => ac.updated_at.as_str(),
        },
    )
    .map_err(|err| format!("failed to put acceptance criterion {}: {err}", ac.id))?;

    replace_ac_concerns(conn, ac)
}

fn reject_empty_slug(ac: &AcceptanceCriterion) -> Result<(), String> {
    if ac.slug.is_empty() {
        return Err(format!(
            "acceptance criterion {}: slug must not be empty",
            ac.id
        ));
    }
    Ok(())
}

fn reject_duplicate_slug_in_spec(conn: &mut Conn, ac: &AcceptanceCriterion) -> Result<(), String> {
    let clash: Option<String> = conn
        .exec_first(
            r"SELECT id FROM acceptance_criteria
              WHERE spec_id = :spec_id AND slug = :slug AND id <> :id",
            params! {
                "spec_id" => ac.spec_id.as_str(),
                "slug" => ac.slug.as_str(),
                "id" => ac.id.as_str(),
            },
        )
        .map_err(|err| format!("failed to check slug uniqueness for {}: {err}", ac.id))?;
    if let Some(other_id) = clash {
        return Err(format!(
            "acceptance criterion slug {:?} is already used in spec {} by {}",
            ac.slug, ac.spec_id, other_id
        ));
    }
    Ok(())
}

fn replace_ac_concerns(conn: &mut Conn, ac: &AcceptanceCriterion) -> Result<(), String> {
    conn.exec_drop(
        "DELETE FROM acceptance_criterion_concerns WHERE ac_id = :ac_id",
        params! {
            "ac_id" => ac.id.as_str(),
        },
    )
    .map_err(|err| format!("failed to reset concerns for {}: {err}", ac.id))?;

    for concern in &ac.concerns {
        conn.exec_drop(
            r"INSERT INTO acceptance_criterion_concerns (ac_id, concern_kind)
              VALUES (:ac_id, :concern_kind)",
            params! {
                "ac_id" => ac.id.as_str(),
                "concern_kind" => concern.as_db_str(),
            },
        )
        .map_err(|err| format!("failed to insert concern for {}: {err}", ac.id))?;
    }
    Ok(())
}

pub fn get_acceptance_criterion(
    conn: &mut Conn,
    ac_id: &str,
) -> Result<Option<AcceptanceCriterion>, String> {
    let row: Option<AcceptanceCriterionRow> = conn
        .exec_first(
            r"SELECT id, spec_id, slug, title, intent, review_mode, risk_level, created_at, updated_at
              FROM acceptance_criteria
              WHERE id = :id",
            params! {
                "id" => ac_id,
            },
        )
        .map_err(|err| format!("failed to get acceptance criterion {ac_id}: {err}"))?;
    row.map(|r| ac_from_row_with_concerns(conn, r)).transpose()
}

pub fn list_acceptance_criteria_for_spec(
    conn: &mut Conn,
    spec_id: &str,
) -> Result<Vec<AcceptanceCriterion>, String> {
    let rows: Vec<AcceptanceCriterionRow> = conn
        .exec(
            r"SELECT id, spec_id, slug, title, intent, review_mode, risk_level, created_at, updated_at
              FROM acceptance_criteria
              WHERE spec_id = :spec_id
              ORDER BY id",
            params! {
                "spec_id" => spec_id,
            },
        )
        .map_err(|err| format!("failed to list ACs for {spec_id}: {err}"))?;

    rows.into_iter()
        .map(|r| ac_from_row_with_concerns(conn, r))
        .collect()
}

fn ac_from_row_with_concerns(
    conn: &mut Conn,
    row: AcceptanceCriterionRow,
) -> Result<AcceptanceCriterion, String> {
    let mut ac = acceptance_criterion_from_row(row)?;
    ac.concerns = concerns_for_ac(conn, &ac.id)?;
    Ok(ac)
}

pub fn put_spec_relation(conn: &mut Conn, relation: &SpecRelation) -> Result<(), String> {
    conn.exec_drop(
        r"INSERT INTO spec_relations (
            id,
            source_spec_id,
            target_spec_id,
            relation_kind,
            note,
            created_at,
            updated_at
          ) VALUES (
            :id,
            :source_spec_id,
            :target_spec_id,
            :relation_kind,
            :note,
            :created_at,
            :updated_at
          )
          ON DUPLICATE KEY UPDATE
            source_spec_id = VALUES(source_spec_id),
            target_spec_id = VALUES(target_spec_id),
            relation_kind = VALUES(relation_kind),
            note = VALUES(note),
            updated_at = VALUES(updated_at)",
        params! {
            "id" => relation.id.as_str(),
            "source_spec_id" => relation.source_spec_id.as_str(),
            "target_spec_id" => relation.target_spec_id.as_str(),
            "relation_kind" => relation.relation_kind.as_str(),
            "note" => relation.note.as_str(),
            "created_at" => "m1",
            "updated_at" => "m1",
        },
    )
    .map_err(|err| format!("failed to put spec relation {}: {err}", relation.id))
}

pub fn list_spec_relations_for_spec(
    conn: &mut Conn,
    spec_id: &str,
) -> Result<Vec<SpecRelation>, String> {
    let rows: Vec<(String, String, String, String, String)> = conn
        .exec(
            r"SELECT id, source_spec_id, target_spec_id, relation_kind, note
              FROM spec_relations
              WHERE source_spec_id = :spec_id OR target_spec_id = :spec_id
              ORDER BY id",
            params! {
                "spec_id" => spec_id,
            },
        )
        .map_err(|err| format!("failed to list relations for {spec_id}: {err}"))?;
    Ok(rows
        .into_iter()
        .map(
            |(id, source_spec_id, target_spec_id, relation_kind, note)| SpecRelation {
                id,
                source_spec_id,
                target_spec_id,
                relation_kind,
                note,
            },
        )
        .collect())
}

fn concerns_for_ac(conn: &mut Conn, ac_id: &str) -> Result<Vec<ConcernKind>, String> {
    let concerns: Vec<String> = conn
        .exec(
            r"SELECT concern_kind
              FROM acceptance_criterion_concerns
              WHERE ac_id = :ac_id
              ORDER BY concern_kind",
            params! {
                "ac_id" => ac_id,
            },
        )
        .map_err(|err| format!("failed to list concerns for {ac_id}: {err}"))?;

    concerns
        .into_iter()
        .map(|value| {
            ConcernKind::from_db_str(&value)
                .ok_or_else(|| format!("unknown concern_kind value in db: {value}"))
        })
        .collect()
}

fn spec_from_row(row: SpecRow) -> Result<Spec, String> {
    let (id, slug, title, description, level, status, created_at, updated_at) = row;
    let level =
        SpecLevel::from_db_str(&level).ok_or_else(|| format!("unknown spec level: {level}"))?;
    let status =
        SpecStatus::from_db_str(&status).ok_or_else(|| format!("unknown spec status: {status}"))?;
    Ok(Spec {
        id,
        slug,
        title,
        description,
        level,
        status,
        created_at,
        updated_at,
    })
}

fn acceptance_criterion_from_row(
    row: AcceptanceCriterionRow,
) -> Result<AcceptanceCriterion, String> {
    let (id, spec_id, slug, title, intent, review_mode, risk_level, created_at, updated_at) = row;
    let review_mode = ReviewMode::from_db_str(&review_mode)
        .ok_or_else(|| format!("unknown review mode: {review_mode}"))?;
    let risk_level = RiskLevel::from_db_str(&risk_level)
        .ok_or_else(|| format!("unknown risk level: {risk_level}"))?;
    Ok(AcceptanceCriterion {
        id,
        spec_id,
        slug,
        title,
        intent,
        review_mode,
        risk_level,
        concerns: Vec::new(),
        created_at,
        updated_at,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use mysql::Conn;

    use crate::db::{self, ConnectionConfig};
    use crate::migrations;
    use crate::models::{AcceptanceCriterion, ConcernKind, Spec, SpecRelation};
    use crate::spec_store;
    use crate::test_world_guard;

    fn maybe_conn() -> Option<test_world_guard::EnvConnLock<Conn>> {
        let lock = test_world_guard::lock_test_env();
        let config = ConnectionConfig::from_env().ok()?;
        test_world_guard::panic_unless_isolated_test_world_for_writes("spec_store::tests", &config);
        migrations::apply_all(&config).ok()?;
        let (conn, _) = db::connect(&config).ok()?;
        Some(test_world_guard::EnvConnLock {
            _lock: lock,
            inner: conn,
        })
    }

    #[test]
    fn spec_round_trip_put_get_list() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let mut spec = Spec::new("SPEC-STORE-1", "Store spec");
        spec.description = "store round-trip".to_string();
        spec.created_at = "t1".to_string();
        spec.updated_at = "t1".to_string();

        spec_store::put_spec(&mut conn, &spec).expect("put_spec");
        let loaded = spec_store::get_spec(&mut conn, "SPEC-STORE-1")
            .expect("get_spec")
            .expect("spec exists");
        assert_eq!(loaded.id, spec.id);
        assert_eq!(loaded.slug, spec.slug);
        assert_eq!(loaded.title, spec.title);

        let all = spec_store::list_specs(&mut conn).expect("list_specs");
        assert!(all.iter().any(|item| item.id == "SPEC-STORE-1"));
    }

    #[test]
    fn acceptance_criterion_round_trip_with_concerns() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let mut spec = Spec::new("SPEC-STORE-2", "Store AC owner");
        spec.description = "owner spec".to_string();
        spec.created_at = "t1".to_string();
        spec.updated_at = "t1".to_string();
        spec_store::put_spec(&mut conn, &spec).expect("put_spec");

        let mut ac = AcceptanceCriterion::new("AC-STORE-1", "SPEC-STORE-2", "AC title");
        ac.intent = "validate concerns round-trip".to_string();
        ac.concerns = vec![ConcernKind::Security, ConcernKind::Performance];
        ac.created_at = "t1".to_string();
        ac.updated_at = "t1".to_string();
        spec_store::put_acceptance_criterion(&mut conn, &ac).expect("put_acceptance_criterion");

        let loaded = spec_store::list_acceptance_criteria_for_spec(&mut conn, "SPEC-STORE-2")
            .expect("list ac");
        let ac_loaded = loaded
            .into_iter()
            .find(|item| item.id == "AC-STORE-1")
            .expect("ac exists");
        assert_eq!(
            ac_loaded.concerns,
            vec![ConcernKind::Performance, ConcernKind::Security]
        );
        assert_eq!(ac_loaded.slug, "ac-store-1");
    }

    #[test]
    fn put_acceptance_criterion_rejects_duplicate_slug_in_same_spec() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let mut spec = Spec::new("SPEC-DUP-SLUG", "Dup slug owner");
        spec.description = "owner spec".to_string();
        spec.created_at = "t1".to_string();
        spec.updated_at = "t1".to_string();
        spec_store::put_spec(&mut conn, &spec).expect("put_spec");

        let mut first = AcceptanceCriterion::new("AC-DUP-A", "SPEC-DUP-SLUG", "First");
        first.slug = "shared-slug".to_string();
        first.intent = "i".to_string();
        first.created_at = "t1".to_string();
        first.updated_at = "t1".to_string();
        spec_store::put_acceptance_criterion(&mut conn, &first).expect("first put");

        let mut second = AcceptanceCriterion::new("AC-DUP-B", "SPEC-DUP-SLUG", "Second");
        second.slug = "shared-slug".to_string();
        second.intent = "i".to_string();
        second.created_at = "t1".to_string();
        second.updated_at = "t1".to_string();

        let err =
            spec_store::put_acceptance_criterion(&mut conn, &second).expect_err("duplicate slug");
        assert!(
            err.contains("shared-slug") && err.contains("AC-DUP-A"),
            "unexpected err: {err}"
        );
    }

    #[test]
    fn spec_relation_round_trip() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let relation = SpecRelation::new(
            "REL-STORE-1",
            "SPEC-STORE-1",
            "SPEC-STORE-2",
            "depends_on",
            "store relation check",
        );
        spec_store::put_spec_relation(&mut conn, &relation).expect("put_spec_relation");

        let relations = spec_store::list_spec_relations_for_spec(&mut conn, "SPEC-STORE-1")
            .expect("list relations");
        assert!(relations.iter().any(|item| item.id == "REL-STORE-1"));
    }

    #[test]
    fn load_spec_graph_sees_specs_acs_and_relations_including_slug() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let mut spec_a = Spec::new("SPEC-GRAPH-A", "graph a");
        spec_a.description = "ga".to_string();
        spec_a.created_at = "t1".to_string();
        spec_a.updated_at = "t1".to_string();
        spec_store::put_spec(&mut conn, &spec_a).expect("put_spec a");

        let mut spec_b = Spec::new("SPEC-GRAPH-B", "graph b");
        spec_b.description = "gb".to_string();
        spec_b.created_at = "t1".to_string();
        spec_b.updated_at = "t1".to_string();
        spec_store::put_spec(&mut conn, &spec_b).expect("put_spec b");

        let mut ac = AcceptanceCriterion::new("AC-GRAPH-1", "SPEC-GRAPH-A", "ac");
        ac.slug = "layout-slug".to_string();
        ac.intent = "i".to_string();
        ac.created_at = "t1".to_string();
        ac.updated_at = "t1".to_string();
        spec_store::put_acceptance_criterion(&mut conn, &ac).expect("put_ac");

        let relation = SpecRelation::new(
            "REL-GRAPH-1",
            "SPEC-GRAPH-A",
            "SPEC-GRAPH-B",
            "depends_on",
            "note",
        );
        spec_store::put_spec_relation(&mut conn, &relation).expect("put_spec_relation");

        let graph = spec_store::load_spec_graph(&mut conn).expect("load_spec_graph");
        assert!(
            graph.specs.iter().any(|s| s.id == "SPEC-GRAPH-A"),
            "spec A present"
        );
        let ac_loaded = graph
            .acceptance_criteria
            .iter()
            .find(|row| row.id == "AC-GRAPH-1")
            .expect("ac in graph");
        assert_eq!(ac_loaded.slug, "layout-slug");
        assert!(
            ac_loaded.concerns.is_empty(),
            "MVP load_spec_graph leaves concerns empty"
        );
        assert!(
            graph.spec_relations.iter().any(|r| r.id == "REL-GRAPH-1"),
            "relation in graph"
        );
    }
}
