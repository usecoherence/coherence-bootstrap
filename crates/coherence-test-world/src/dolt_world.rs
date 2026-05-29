use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;

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

CREATE TABLE IF NOT EXISTS acceptance_criterion_concerns (
    id VARCHAR(255) PRIMARY KEY,
    acceptance_criterion_id VARCHAR(255) NOT NULL,
    concern TEXT NOT NULL,
    created_at VARCHAR(50) NOT NULL DEFAULT '',
    updated_at VARCHAR(50) NOT NULL DEFAULT '',
    FOREIGN KEY (acceptance_criterion_id) REFERENCES acceptance_criteria(id)
);

CREATE TABLE IF NOT EXISTS spec_relations (
    id VARCHAR(255) PRIMARY KEY,
    source_spec_id VARCHAR(255) NOT NULL,
    target_spec_id VARCHAR(255) NOT NULL,
    relation_kind VARCHAR(50) NOT NULL DEFAULT 'depends_on',
    note TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (source_spec_id) REFERENCES specs(id),
    FOREIGN KEY (target_spec_id) REFERENCES specs(id)
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
        dolt_ok(&data_dir, &["sql", "-q", &format!("USE {db_name}; {MIGRATION_SPECS}")])?;

        Ok(Self { data_dir, db_name })
    }

    pub fn try_init(db_name: &str) -> Result<Self, String> {
        if Command::new("dolt").arg("--version").output().is_err() {
            return Err("dolt CLI not found — Dolt world unavailable".to_string());
        }
        Self::init(db_name)
    }

    pub fn run_sql(&self, query: &str) -> Result<String, String> {
        let query = format!("USE {}; {query}", self.db_name);
        let out = dolt_in(&self.data_dir, &["sql", "-q", &query])?;
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

impl DoltWorld {
    pub fn start_server(&self, socket: &Path) -> Result<DoltServer, String> {
        let port = pick_unused_port();
        let mut child = Command::new("dolt")
            .args([
                "sql-server",
                "--data-dir",
                &self.data_dir.to_string_lossy(),
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--socket",
                &socket.to_string_lossy(),
            ])
            .spawn()
            .map_err(|e| format!("start dolt sql-server: {e}"))?;

        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(15);
        loop {
            if socket.exists() {
                let ping = Command::new("dolt")
                    .args(["sql", "-q", "SELECT 1"])
                    .env("DOLT_SOCKET", socket.to_string_lossy().as_ref())
                    .output();
                if let Ok(out) = ping {
                    if out.status.success() {
                        return Ok(DoltServer { child, socket_path: socket.to_path_buf() });
                    }
                }
            }
            if start.elapsed() > timeout {
                let _ = child.kill();
                let _ = child.wait();
                return Err("dolt sql-server did not become ready within 15s".into());
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

impl Drop for DoltWorld {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.data_dir);
    }
}

fn pick_unused_port() -> u16 {
    // let the OS assign a port, then close it immediately
    if let Ok(listener) = std::net::TcpListener::bind("127.0.0.1:0") {
        if let Ok(addr) = listener.local_addr() {
            return addr.port();
        }
    }
    43306 // fallback
}

#[derive(Debug)]
pub struct DoltServer {
    child: Child,
    socket_path: PathBuf,
}

impl DoltServer {
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

impl Drop for DoltServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
