use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct DoltWorld {
    data_dir: PathBuf,
    db_name: String,
}

static MIGRATION_SPECS: &str = r"
CREATE TABLE IF NOT EXISTS specs (
    id VARCHAR(255) PRIMARY KEY,
    slug VARCHAR(255) NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    level VARCHAR(50) NOT NULL DEFAULT 'module',
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    created_at VARCHAR(50) NOT NULL DEFAULT '',
    updated_at VARCHAR(50) NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS acceptance_criteria (
    id VARCHAR(255) PRIMARY KEY,
    spec_id VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL,
    title VARCHAR(255) NOT NULL,
    intent TEXT NOT NULL DEFAULT '',
    review_mode VARCHAR(50) NOT NULL DEFAULT 'manual',
    risk_level VARCHAR(50) NOT NULL DEFAULT 'medium',
    created_at VARCHAR(50) NOT NULL DEFAULT '',
    updated_at VARCHAR(50) NOT NULL DEFAULT '',
    FOREIGN KEY (spec_id) REFERENCES specs(id)
);
";

fn dolt_in(dir: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("dolt")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("dolt: {e}"))
}

fn dolt_ok(dir: &Path, args: &[&str]) -> Result<(), String> {
    let out = dolt_in(dir, args)?;
    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        Err(format!("dolt {} failed: {stderr}", args[0]))
    }
}

impl DoltWorld {
    pub fn init(db_name: &str) -> Result<Self, String> {
        let data_dir = Self::create_temp_dir()?;
        let db_name = db_name.to_string();

        dolt_ok(&data_dir, &["init"])?;
        dolt_ok(&data_dir, &["sql", "-q", &format!("CREATE DATABASE IF NOT EXISTS {db_name}")])?;
        dolt_ok(&data_dir, &["sql", "-q", MIGRATION_SPECS])?;

        Ok(Self { data_dir, db_name })
    }

    pub fn try_init(db_name: &str) -> Result<Self, String> {
        if Command::new("dolt").arg("--version").output().is_err() {
            return Err("dolt CLI not found — Dolt world unavailable".to_string());
        }
        Self::init(db_name)
    }

    pub fn run_sql(&self, query: &str) -> Result<String, String> {
        let out = dolt_in(&self.data_dir, &["sql", "-q", query])?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(format!("dolt sql failed: {stderr}"))
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn db_name(&self) -> &str {
        &self.db_name
    }

    fn create_temp_dir() -> Result<PathBuf, String> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "coherence_test_dolt_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("create temp dolt dir: {e}"))?;
        Ok(path)
    }
}

impl Drop for DoltWorld {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.data_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dolt_world_init_creates_data_dir() {
        let dw = DoltWorld::init("test_db").unwrap();
        assert!(dw.data_dir().exists());
    }

    #[test]
    fn dolt_world_run_sql_works() {
        let dw = DoltWorld::init("test_db").unwrap();
        let result = dw.run_sql("SELECT COUNT(*) AS c FROM specs");
        assert!(result.is_ok());
    }

    #[test]
    fn dolt_world_cleanup_on_drop() {
        let path;
        {
            let dw = DoltWorld::init("test_db").unwrap();
            path = dw.data_dir().to_path_buf();
            assert!(path.exists());
        }
        assert!(!path.exists());
    }
}
