#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecLevel {
    Product,
    System,
    Module,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecStatus {
    Draft,
    Active,
    Deprecated,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewMode {
    Manual,
    Automated,
    Hybrid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcernKind {
    Correctness,
    Security,
    Performance,
    Reliability,
    Maintainability,
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
        Self {
            id: id.into(),
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

fn slug_from_id(id: &str) -> String {
    id.to_ascii_lowercase().replace('_', "-")
}

#[cfg(test)]
mod tests {
    use super::{
        AcceptanceCriterion, ConcernKind, ReviewMode, RiskLevel, Spec, SpecLevel, SpecRelation,
        SpecStatus,
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
        assert_eq!(ac.title, "AC title");
        assert_eq!(ac.intent, "");
        assert_eq!(ac.review_mode, ReviewMode::Manual);
        assert_eq!(ac.risk_level, RiskLevel::Medium);
        assert_eq!(ac.concerns, Vec::<ConcernKind>::new());
        assert_eq!(ac.created_at, "");
        assert_eq!(ac.updated_at, "");
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
}
