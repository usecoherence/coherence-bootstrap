//! Persistence for codeintel code locations and AC↔code links (`codeintel_*` tables).
//! Does not run tests or shell commands — storage only.
//! Table/schema ownership narrative: see `AGENTS.md` § M1 module ownership.

use mysql::prelude::Queryable;
use mysql::{params, Conn};

use crate::models::{
    AcCodeLink, AcCodeLinkWithLocation, AcCodeRelationKind, CodeLocation, CodeLocationKind,
};

pub fn put_code_location(conn: &mut Conn, loc: &CodeLocation) -> Result<(), String> {
    conn.exec_drop(
        r"INSERT INTO codeintel_code_locations (
            id,
            repo_path,
            file_path,
            kind,
            symbol,
            test_command,
            created_at,
            updated_at
          ) VALUES (
            :id,
            :repo_path,
            :file_path,
            :kind,
            :symbol,
            :test_command,
            :created_at,
            :updated_at
          )
          ON DUPLICATE KEY UPDATE
            repo_path = VALUES(repo_path),
            file_path = VALUES(file_path),
            kind = VALUES(kind),
            symbol = VALUES(symbol),
            test_command = VALUES(test_command),
            updated_at = VALUES(updated_at)",
        params! {
            "id" => loc.id.as_str(),
            "repo_path" => loc.repo_path.as_str(),
            "file_path" => loc.file_path.as_str(),
            "kind" => loc.kind.as_db_str(),
            "symbol" => loc.symbol.clone(),
            "test_command" => loc.test_command.clone(),
            "created_at" => loc.created_at.as_str(),
            "updated_at" => loc.updated_at.as_str(),
        },
    )
    .map_err(|err| format!("failed to put code location {}: {err}", loc.id))
}

pub fn get_code_location(conn: &mut Conn, id: &str) -> Result<Option<CodeLocation>, String> {
    let row: Option<(
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        String,
    )> = conn
        .exec_first(
            r"SELECT id, repo_path, file_path, kind, symbol, test_command, created_at, updated_at
              FROM codeintel_code_locations
              WHERE id = :id",
            params! {
                "id" => id,
            },
        )
        .map_err(|err| format!("failed to get code location {id}: {err}"))?;

    row.map(code_location_from_row).transpose()
}

pub fn put_ac_code_link(conn: &mut Conn, link: &AcCodeLink) -> Result<(), String> {
    conn.exec_drop(
        r"INSERT INTO codeintel_ac_links (
            id,
            ac_id,
            code_location_id,
            relation_kind,
            note,
            created_at,
            updated_at
          ) VALUES (
            :id,
            :ac_id,
            :code_location_id,
            :relation_kind,
            :note,
            :created_at,
            :updated_at
          )
          ON DUPLICATE KEY UPDATE
            ac_id = VALUES(ac_id),
            code_location_id = VALUES(code_location_id),
            relation_kind = VALUES(relation_kind),
            note = VALUES(note),
            updated_at = VALUES(updated_at)",
        params! {
            "id" => link.id.as_str(),
            "ac_id" => link.ac_id.as_str(),
            "code_location_id" => link.code_location_id.as_str(),
            "relation_kind" => link.relation_kind.as_db_str(),
            "note" => link.note.as_str(),
            "created_at" => link.created_at.as_str(),
            "updated_at" => link.updated_at.as_str(),
        },
    )
    .map_err(|err| format!("failed to put AC code link {}: {err}", link.id))
}

/// Lists all links for an AC with joined [`CodeLocation`] rows (stable order by link id).
pub fn list_code_links_for_ac(
    conn: &mut Conn,
    ac_id: &str,
) -> Result<Vec<AcCodeLinkWithLocation>, String> {
    type LinkRow = (String, String, String, String, String, String, String);
    let link_rows: Vec<LinkRow> = conn
        .exec(
            r"SELECT id, ac_id, code_location_id, relation_kind, note, created_at, updated_at
              FROM codeintel_ac_links
              WHERE ac_id = :ac_id
              ORDER BY id",
            params! {
                "ac_id" => ac_id,
            },
        )
        .map_err(|err| format!("failed to list code links for AC {ac_id}: {err}"))?;

    let mut out = Vec::with_capacity(link_rows.len());
    for row in link_rows {
        let (id, lac_id, code_location_id, relation_kind, note, created_at, updated_at) = row;
        let link = ac_code_link_from_columns(
            id,
            lac_id,
            code_location_id,
            relation_kind,
            note,
            created_at,
            updated_at,
        )?;
        let location = get_code_location(conn, &link.code_location_id)?
            .ok_or_else(|| format!("missing code location {}", link.code_location_id))?;
        out.push(AcCodeLinkWithLocation { link, location });
    }
    Ok(out)
}

fn code_location_from_row(
    row: (
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        String,
    ),
) -> Result<CodeLocation, String> {
    let (id, repo_path, file_path, kind, symbol, test_command, created_at, updated_at) = row;
    code_location_from_columns(
        id,
        repo_path,
        file_path,
        kind,
        symbol,
        test_command,
        created_at,
        updated_at,
    )
}

fn code_location_from_columns(
    id: String,
    repo_path: String,
    file_path: String,
    kind: String,
    symbol: Option<String>,
    test_command: Option<String>,
    created_at: String,
    updated_at: String,
) -> Result<CodeLocation, String> {
    let kind = CodeLocationKind::from_db_str(&kind)
        .ok_or_else(|| format!("unknown code location kind: {kind}"))?;
    Ok(CodeLocation {
        id,
        repo_path,
        file_path,
        kind,
        symbol,
        test_command,
        created_at,
        updated_at,
    })
}

fn ac_code_link_from_columns(
    id: String,
    ac_id: String,
    code_location_id: String,
    relation_kind: String,
    note: String,
    created_at: String,
    updated_at: String,
) -> Result<AcCodeLink, String> {
    let relation_kind = AcCodeRelationKind::from_db_str(&relation_kind)
        .ok_or_else(|| format!("unknown AC code relation kind: {relation_kind}"))?;
    Ok(AcCodeLink {
        id,
        ac_id,
        code_location_id,
        relation_kind,
        note,
        created_at,
        updated_at,
    })
}

#[cfg(test)]
mod tests {
    use mysql::Conn;

    use crate::ac_code_link_store;
    use crate::db::{self, ConnectionConfig};
    use crate::migrations;
    use crate::models::{
        AcCodeLink, AcCodeRelationKind, AcceptanceCriterion, CodeLocation, CodeLocationKind, Spec,
    };
    use crate::spec_store;
    use crate::test_world_guard;

    fn maybe_conn() -> Option<Conn> {
        let config = ConnectionConfig::from_env();
        test_world_guard::panic_unless_isolated_test_world_for_writes(
            "ac_code_link_store::tests",
            &config,
        );
        let _ = migrations::apply_all(&config).ok()?;
        db::connect(&config).ok().map(|(conn, _)| conn)
    }

    fn unique_label(prefix: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{prefix}-{nanos}-{}", std::process::id())
    }

    #[test]
    fn code_location_put_get_round_trip() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let id = unique_label("CI-LOC-RT");
        let mut loc = CodeLocation::new(
            id.clone(),
            "/repo/coherence-core-db",
            "crates/coherence-core-db/src/lib.rs",
        );
        loc.kind = CodeLocationKind::TestFile;
        loc.symbol = Some("my_fn".to_string());
        loc.created_at = "t-loc-1".to_string();
        loc.updated_at = "t-loc-1".to_string();

        ac_code_link_store::put_code_location(&mut conn, &loc).expect("put_code_location");
        let loaded = ac_code_link_store::get_code_location(&mut conn, &id)
            .expect("get_code_location")
            .expect("exists");
        assert_eq!(loaded, loc);
    }

    #[test]
    fn ac_code_link_put_and_list_with_location() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let suf = unique_label("CI-SUF");
        let loc_id = format!("CI-LOC-{suf}");
        let ac_id = format!("CI-AC-{suf}");
        let link1_id = format!("CI-LINK-A-{suf}");
        let link2_id = format!("CI-LINK-B-{suf}");

        let mut loc = CodeLocation::new(loc_id.clone(), "/repo/x", "tests/foo.rs");
        loc.kind = CodeLocationKind::TestCommand;
        loc.test_command = Some("cargo test -p x foo".to_string());
        loc.created_at = "t1".to_string();
        loc.updated_at = "t1".to_string();
        ac_code_link_store::put_code_location(&mut conn, &loc).expect("put_code_location");

        let mut link = AcCodeLink::new(
            link1_id,
            ac_id.clone(),
            loc_id.clone(),
            AcCodeRelationKind::VerifiedBy,
        );
        link.note = "verification".to_string();
        link.created_at = "t2".to_string();
        link.updated_at = "t2".to_string();
        ac_code_link_store::put_ac_code_link(&mut conn, &link).expect("put_ac_code_link");

        let list = ac_code_link_store::list_code_links_for_ac(&mut conn, &ac_id).expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].link, link);
        assert_eq!(list[0].location, loc);

        let mut link2 = AcCodeLink::new(
            link2_id,
            ac_id.clone(),
            loc_id,
            AcCodeRelationKind::VerifiedBy,
        );
        link2.note = "second".to_string();
        link2.created_at = "t3".to_string();
        link2.updated_at = "t3".to_string();
        ac_code_link_store::put_ac_code_link(&mut conn, &link2).expect("put_ac_code_link");

        let list2 = ac_code_link_store::list_code_links_for_ac(&mut conn, &ac_id).expect("list2");
        assert_eq!(list2.len(), 2);
    }

    #[test]
    fn verified_by_round_trip_with_spec_and_ac() {
        let Some(mut conn) = maybe_conn() else {
            return;
        };

        let suf = unique_label("CI-VFY");
        let spec_id = format!("SPEC-CI-{suf}");
        let ac_id = format!("AC-CI-{suf}");
        let loc_id = format!("CI-LOC-AC-{suf}");
        let link_id = format!("CI-LINK-AC-{suf}");

        let mut spec = Spec::new(spec_id.clone(), "Codeintel owner");
        spec.description = "ci link store".to_string();
        spec.created_at = "ts".to_string();
        spec.updated_at = "ts".to_string();
        spec_store::put_spec(&mut conn, &spec).expect("put_spec");

        let mut ac = AcceptanceCriterion::new(ac_id.clone(), spec_id, "Criterion");
        ac.intent = "verified_by test command".to_string();
        ac.created_at = "ta".to_string();
        ac.updated_at = "ta".to_string();
        spec_store::put_acceptance_criterion(&mut conn, &ac).expect("put_ac");

        let mut loc = CodeLocation::new(loc_id.clone(), "/repo/coherence-core-db", ".");
        loc.kind = CodeLocationKind::TestCommand;
        loc.test_command = Some("make tool run".to_string());
        loc.created_at = "tl".to_string();
        loc.updated_at = "tl".to_string();
        ac_code_link_store::put_code_location(&mut conn, &loc).expect("put_code_location");

        let mut link = AcCodeLink::new(
            link_id,
            ac_id.clone(),
            loc_id,
            AcCodeRelationKind::VerifiedBy,
        );
        link.note = String::new();
        link.created_at = "tln".to_string();
        link.updated_at = "tln".to_string();
        ac_code_link_store::put_ac_code_link(&mut conn, &link).expect("put_ac_code_link");

        let rows = ac_code_link_store::list_code_links_for_ac(&mut conn, &ac_id).expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].link.relation_kind, AcCodeRelationKind::VerifiedBy);
        assert_eq!(
            rows[0].location.test_command.as_deref(),
            Some("make tool run")
        );
    }
}
