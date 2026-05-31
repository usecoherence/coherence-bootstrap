use std::path::PathBuf;

use coherence_core_db::ac_verification_store::AcVerificationLatest;
use coherence_core_db::ac_verify::{AcVerifyAcRunResult, VerifySpecRunResult};
use coherence_core_db::models::{AcceptanceCriterion, Spec, SpecGraph};

pub trait SpecRepository {
    fn load_spec_graph(&mut self) -> Result<SpecGraph, String>;
    fn get_spec(&mut self, id: &str) -> Result<Option<Spec>, String>;
    fn put_spec(&mut self, spec: &Spec) -> Result<(), String>;
    fn get_acceptance_criterion(&mut self, id: &str)
        -> Result<Option<AcceptanceCriterion>, String>;
    fn put_acceptance_criterion(&mut self, ac: &AcceptanceCriterion) -> Result<(), String>;
    fn verify_acceptance_criterion(&mut self, ac_id: &str) -> Result<AcVerifyAcRunResult, String>;
    fn verify_spec(&mut self, spec_id: &str) -> Result<VerifySpecRunResult, String>;
    fn get_ac_verification_latest(
        &mut self,
        ac_id: &str,
    ) -> Result<Option<AcVerificationLatest>, String>;
}

pub struct DoltSpecRepository {
    conn: mysql::Conn,
    project_dir: PathBuf,
}

impl DoltSpecRepository {
    pub fn new(project_dir: PathBuf) -> Result<Self, String> {
        let orig = std::env::current_dir().map_err(|e| e.to_string())?;
        let restore = || {
            let _ = std::env::set_current_dir(&orig);
        };
        let _ = std::env::set_current_dir(&project_dir).map_err(|e| e.to_string())?;

        let config = coherence_core_db::db::ConnectionConfig::from_env().map_err(|e| {
            restore();
            format!("config: {e}")
        })?;

        let (conn, _) = coherence_core_db::db::connect(&config).map_err(|e| {
            restore();
            format!("connect: {e}")
        })?;

        restore();
        Ok(Self { conn, project_dir })
    }

    fn with_project_dir<T>(
        &mut self,
        f: impl FnOnce(&mut mysql::Conn) -> Result<T, String>,
    ) -> Result<T, String> {
        let orig = std::env::current_dir().map_err(|e| e.to_string())?;
        std::env::set_current_dir(&self.project_dir).map_err(|e| e.to_string())?;
        let result = f(&mut self.conn);
        let restore_result = std::env::set_current_dir(&orig).map_err(|e| e.to_string());
        match (result, restore_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(err), _) => Err(err),
            (Ok(_), Err(err)) => Err(err),
        }
    }
}

impl SpecRepository for DoltSpecRepository {
    fn load_spec_graph(&mut self) -> Result<SpecGraph, String> {
        coherence_core_db::spec_store::load_spec_graph(&mut self.conn).map_err(|e| e.to_string())
    }

    fn get_spec(&mut self, id: &str) -> Result<Option<Spec>, String> {
        coherence_core_db::spec_store::get_spec(&mut self.conn, id).map_err(|e| e.to_string())
    }

    fn put_spec(&mut self, spec: &Spec) -> Result<(), String> {
        coherence_core_db::spec_store::put_spec(&mut self.conn, spec).map_err(|e| e.to_string())
    }

    fn get_acceptance_criterion(
        &mut self,
        id: &str,
    ) -> Result<Option<AcceptanceCriterion>, String> {
        coherence_core_db::spec_store::get_acceptance_criterion(&mut self.conn, id)
            .map_err(|e| e.to_string())
    }

    fn put_acceptance_criterion(&mut self, ac: &AcceptanceCriterion) -> Result<(), String> {
        coherence_core_db::spec_store::put_acceptance_criterion(&mut self.conn, ac)
            .map_err(|e| e.to_string())
    }

    fn verify_acceptance_criterion(&mut self, ac_id: &str) -> Result<AcVerifyAcRunResult, String> {
        self.with_project_dir(|conn| {
            coherence_core_db::ac_verify::verify_acceptance_criterion(conn, ac_id)
        })
    }

    fn verify_spec(&mut self, spec_id: &str) -> Result<VerifySpecRunResult, String> {
        self.with_project_dir(|conn| coherence_core_db::ac_verify::verify_spec(conn, spec_id))
    }

    fn get_ac_verification_latest(
        &mut self,
        ac_id: &str,
    ) -> Result<Option<AcVerificationLatest>, String> {
        coherence_core_db::ac_verification_store::get_ac_verification_latest(&mut self.conn, ac_id)
    }
}
