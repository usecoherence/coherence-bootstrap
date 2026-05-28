#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecLevel {
    Product,
    System,
    Module,
}

impl SpecLevel {
    #[must_use]
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Product => "product",
            Self::System => "system",
            Self::Module => "module",
        }
    }

    #[must_use]
    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "product" => Some(Self::Product),
            "system" => Some(Self::System),
            "module" => Some(Self::Module),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecStatus {
    Draft,
    Active,
    Deprecated,
    Archived,
}

impl SpecStatus {
    #[must_use]
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Deprecated => "deprecated",
            Self::Archived => "archived",
        }
    }

    #[must_use]
    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(Self::Draft),
            "active" => Some(Self::Active),
            "deprecated" => Some(Self::Deprecated),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewMode {
    Manual,
    Automated,
    Hybrid,
}

impl ReviewMode {
    #[must_use]
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Automated => "automated",
            Self::Hybrid => "hybrid",
        }
    }

    #[must_use]
    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "manual" => Some(Self::Manual),
            "automated" => Some(Self::Automated),
            "hybrid" => Some(Self::Hybrid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    #[must_use]
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    #[must_use]
    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcernKind {
    Correctness,
    Security,
    Performance,
    Reliability,
    Maintainability,
}

impl ConcernKind {
    #[must_use]
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Correctness => "correctness",
            Self::Security => "security",
            Self::Performance => "performance",
            Self::Reliability => "reliability",
            Self::Maintainability => "maintainability",
        }
    }

    #[must_use]
    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "correctness" => Some(Self::Correctness),
            "security" => Some(Self::Security),
            "performance" => Some(Self::Performance),
            "reliability" => Some(Self::Reliability),
            "maintainability" => Some(Self::Maintainability),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub level: SpecLevel,
    pub status: SpecStatus,
    pub created_at: String,
    pub updated_at: String,
}

impl Spec {
    #[must_use]
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let id = id.into();
        let title = title.into();
        Self {
            slug: slug_from_id(&id),
            id,
            title,
            description: String::new(),
            level: SpecLevel::Module,
            status: SpecStatus::Draft,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceCriterion {
    pub id: String,
    pub spec_id: String,
    /// Filesystem-oriented segment; default mirrors `id` via [`slug_from_id`].
    pub slug: String,
    pub title: String,
    pub intent: String,
    pub review_mode: ReviewMode,
    pub risk_level: RiskLevel,
    pub concerns: Vec<ConcernKind>,
    pub created_at: String,
    pub updated_at: String,
}

impl AcceptanceCriterion {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        spec_id: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        let id = id.into();
        Self {
            slug: slug_from_id(&id),
            id,
            spec_id: spec_id.into(),
            title: title.into(),
            intent: String::new(),
            review_mode: ReviewMode::Manual,
            risk_level: RiskLevel::Medium,
            concerns: Vec::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecRelation {
    pub id: String,
    pub source_spec_id: String,
    pub target_spec_id: String,
    pub relation_kind: String,
    pub note: String,
}

impl SpecRelation {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        source_spec_id: impl Into<String>,
        target_spec_id: impl Into<String>,
        relation_kind: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source_spec_id: source_spec_id.into(),
            target_spec_id: target_spec_id.into(),
            relation_kind: relation_kind.into(),
            note: note.into(),
        }
    }
}

/// Full spec-module row snapshot for layout tooling and other bulk consumers.
///
/// **MVP:** [`crate::spec_store::load_spec_graph`] does not validate graph consistency (orphan
/// [`SpecRelation`] rows, missing specs on edges, dangling [`AcceptanceCriterion::spec_id`], etc.).
/// **`AcceptanceCriterion.concerns`** are left empty in this snapshot path (three-table load only);
/// use [`crate::spec_store::get_acceptance_criterion`] or
/// [`crate::spec_store::list_acceptance_criteria_for_spec`] when concern rows matter.
#[allow(dead_code)] // Bulk snapshot surface for embedders / layout tooling (not CLI-wired yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecGraph {
    pub specs: Vec<Spec>,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub spec_relations: Vec<SpecRelation>,
}

#[allow(dead_code)]
impl SpecGraph {
    #[must_use]
    pub fn new(
        specs: Vec<Spec>,
        acceptance_criteria: Vec<AcceptanceCriterion>,
        spec_relations: Vec<SpecRelation>,
    ) -> Self {
        Self {
            specs,
            acceptance_criteria,
            spec_relations,
        }
    }
}

pub fn slug_from_id(id: &str) -> String {
    id.to_ascii_lowercase().replace('_', "-")
}

/// Kind of stored code location (`codeintel_code_locations.kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeLocationKind {
    /// Path to a test file (verified-by file target).
    TestFile,
    /// A runnable test command (e.g. `cargo test ...`).
    TestCommand,
}

impl CodeLocationKind {
    #[must_use]
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::TestFile => "test_file",
            Self::TestCommand => "test_command",
        }
    }

    #[must_use]
    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "test_file" => Some(Self::TestFile),
            "test_command" => Some(Self::TestCommand),
            _ => None,
        }
    }
}

/// Relation between an acceptance criterion and a code location (`codeintel_ac_links.relation_kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcCodeRelationKind {
    VerifiedBy,
    ImplementedBy,
    TouchedBy,
}

impl AcCodeRelationKind {
    #[must_use]
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::VerifiedBy => "verified_by",
            Self::ImplementedBy => "implemented_by",
            Self::TouchedBy => "touched_by",
        }
    }

    #[must_use]
    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "verified_by" => Some(Self::VerifiedBy),
            "implemented_by" => Some(Self::ImplementedBy),
            "touched_by" => Some(Self::TouchedBy),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeLocation {
    pub id: String,
    pub repo_path: String,
    pub file_path: String,
    pub kind: CodeLocationKind,
    pub symbol: Option<String>,
    pub test_command: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl CodeLocation {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        repo_path: impl Into<String>,
        file_path: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            repo_path: repo_path.into(),
            file_path: file_path.into(),
            kind: CodeLocationKind::TestFile,
            symbol: None,
            test_command: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcCodeLink {
    pub id: String,
    pub ac_id: String,
    pub code_location_id: String,
    pub relation_kind: AcCodeRelationKind,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

impl AcCodeLink {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        ac_id: impl Into<String>,
        code_location_id: impl Into<String>,
        relation_kind: AcCodeRelationKind,
    ) -> Self {
        Self {
            id: id.into(),
            ac_id: ac_id.into(),
            code_location_id: code_location_id.into(),
            relation_kind,
            note: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

/// Link row plus [`CodeLocation`], for consumers that need paths and `test_command` without a second query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcCodeLinkWithLocation {
    pub link: AcCodeLink,
    pub location: CodeLocation,
}

#[cfg(test)]
mod tests {
    use super::{
        AcceptanceCriterion, ConcernKind, ReviewMode, RiskLevel, Spec, SpecGraph, SpecLevel,
        SpecRelation, SpecStatus,
    };

    #[test]
    fn spec_constructor_sets_fields() {
        let spec = Spec::new("SPEC-1", "Spec title");
        assert_eq!(spec.id, "SPEC-1");
        assert_eq!(spec.slug, "spec-1");
        assert_eq!(spec.title, "Spec title");
        assert_eq!(spec.description, "");
        assert_eq!(spec.level, SpecLevel::Module);
        assert_eq!(spec.status, SpecStatus::Draft);
        assert_eq!(spec.created_at, "");
        assert_eq!(spec.updated_at, "");
    }

    #[test]
    fn acceptance_criterion_constructor_sets_fields() {
        let ac = AcceptanceCriterion::new("AC-1", "SPEC-1", "AC title");
        assert_eq!(ac.id, "AC-1");
        assert_eq!(ac.spec_id, "SPEC-1");
        assert_eq!(ac.slug, "ac-1");
        assert_eq!(ac.title, "AC title");
        assert_eq!(ac.intent, "");
        assert_eq!(ac.review_mode, ReviewMode::Manual);
        assert_eq!(ac.risk_level, RiskLevel::Medium);
        assert_eq!(ac.concerns, Vec::<ConcernKind>::new());
        assert_eq!(ac.created_at, "");
        assert_eq!(ac.updated_at, "");
    }

    #[test]
    fn acceptance_criterion_slug_from_id_normalizes_underscores() {
        let ac = AcceptanceCriterion::new("AC_SMOKE_CASE", "SPEC-1", "t");
        assert_eq!(ac.slug, "ac-smoke-case");
    }

    #[test]
    fn acceptance_criterion_explicit_slug_uses_slug_from_id_rules() {
        let mut ac = AcceptanceCriterion::new("AC-1", "SPEC-1", "t");
        ac.slug = super::slug_from_id("MY_CUSTOM_Slug");
        assert_eq!(ac.slug, "my-custom-slug");
    }

    #[test]
    fn spec_relation_constructor_sets_fields() {
        let relation = SpecRelation::new(
            "REL-1",
            "SPEC-1",
            "SPEC-2",
            "depends_on",
            "spec 1 depends on spec 2",
        );
        assert_eq!(relation.id, "REL-1");
        assert_eq!(relation.source_spec_id, "SPEC-1");
        assert_eq!(relation.target_spec_id, "SPEC-2");
        assert_eq!(relation.relation_kind, "depends_on");
        assert_eq!(relation.note, "spec 1 depends on spec 2");
    }

    #[test]
    fn spec_graph_new_holds_vectors() {
        let specs = vec![Spec::new("SPEC-A", "a"), Spec::new("SPEC-B", "b")];
        let acs = vec![
            AcceptanceCriterion::new("AC-1", "SPEC-A", "ac"),
            AcceptanceCriterion::new("AC-2", "SPEC-B", "ac2"),
        ];
        let rels = vec![SpecRelation::new(
            "REL-1",
            "SPEC-A",
            "SPEC-B",
            "depends_on",
            "",
        )];
        let g = SpecGraph::new(specs.clone(), acs.clone(), rels.clone());
        assert_eq!(g.specs, specs);
        assert_eq!(g.acceptance_criteria, acs);
        assert_eq!(g.spec_relations, rels);
    }
}
