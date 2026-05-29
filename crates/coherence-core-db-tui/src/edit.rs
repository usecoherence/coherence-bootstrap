use coherence_core_db::models::{
    AcceptanceCriterion, ReviewMode, RiskLevel, Spec, SpecLevel, SpecStatus,
};

#[derive(Debug, Clone)]
pub enum Draft {
    Spec {
        spec_id: String,
        original_status: SpecStatus,
        original_level: SpecLevel,
        original_description: String,
        pending_status: SpecStatus,
        pending_level: SpecLevel,
        pending_description: Option<String>,
    },
    Ac {
        ac_id: String,
        original_review_mode: ReviewMode,
        original_risk_level: RiskLevel,
        original_intent: String,
        pending_review_mode: ReviewMode,
        pending_risk_level: RiskLevel,
        pending_intent: Option<String>,
    },
}

impl Draft {
    pub fn from_spec(spec: &Spec) -> Self {
        Self::Spec {
            spec_id: spec.id.clone(),
            original_status: spec.status,
            original_level: spec.level,
            original_description: spec.description.clone(),
            pending_status: spec.status,
            pending_level: spec.level,
            pending_description: None,
        }
    }

    pub fn from_ac(ac: &AcceptanceCriterion) -> Self {
        Self::Ac {
            ac_id: ac.id.clone(),
            original_review_mode: ac.review_mode,
            original_risk_level: ac.risk_level,
            original_intent: ac.intent.clone(),
            pending_review_mode: ac.review_mode,
            pending_risk_level: ac.risk_level,
            pending_intent: None,
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self {
            Self::Spec {
                original_status,
                original_level,
                original_description,
                pending_status,
                pending_level,
                pending_description,
                ..
            } => {
                pending_status != original_status
                    || pending_level != original_level
                    || pending_description
                        .as_ref()
                        .is_some_and(|d| d != original_description)
            }
            Self::Ac {
                original_review_mode,
                original_risk_level,
                original_intent,
                pending_review_mode,
                pending_risk_level,
                pending_intent,
                ..
            } => {
                pending_review_mode != original_review_mode
                    || pending_risk_level != original_risk_level
                    || pending_intent.as_ref().is_some_and(|i| i != original_intent)
            }
        }
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        match self {
            Self::Spec {
                pending_description, ..
            } => {
                if let Some(desc) = pending_description {
                    if desc.trim().is_empty() {
                        errors.push("Description must not be empty".into());
                    }
                }
            }
            Self::Ac {
                pending_intent, ..
            } => {
                if let Some(intent) = pending_intent {
                    if intent.trim().is_empty() {
                        errors.push("Intent must not be empty".into());
                    }
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
