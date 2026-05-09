#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub id: String,
    pub title: String,
}

impl Spec {
    #[must_use]
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceCriterion {
    pub id: String,
    pub spec_id: String,
    pub title: String,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AcceptanceCriterion, Spec};

    #[test]
    fn spec_constructor_sets_fields() {
        let spec = Spec::new("SPEC-1", "Spec title");
        assert_eq!(spec.id, "SPEC-1");
        assert_eq!(spec.title, "Spec title");
    }

    #[test]
    fn acceptance_criterion_constructor_sets_fields() {
        let ac = AcceptanceCriterion::new("AC-1", "SPEC-1", "AC title");
        assert_eq!(ac.id, "AC-1");
        assert_eq!(ac.spec_id, "SPEC-1");
        assert_eq!(ac.title, "AC title");
    }
}
