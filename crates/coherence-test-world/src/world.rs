use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::dolt_world::DoltWorld;
use crate::recipe::E2eRecipe;
use crate::scaffold::Scaffold;

#[derive(Debug, Clone)]
pub struct Evidence {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub file_snapshots: HashMap<String, String>,
}

impl Evidence {
    pub fn new() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            file_snapshots: HashMap::new(),
        }
    }

    pub fn record_command(mut self, output: &std::process::Output) -> Self {
        self.stdout = String::from_utf8_lossy(&output.stdout).to_string();
        self.stderr = String::from_utf8_lossy(&output.stderr).to_string();
        self.exit_code = output.status.code();
        self
    }

    pub fn snapshot_file(mut self, path: &str, content: &str) -> Self {
        self.file_snapshots
            .insert(path.to_string(), content.to_string());
        self
    }

    pub fn write_to_dir(&self, dir: &std::path::Path) -> Result<(), String> {
        let meta = serde_json::json!({
            "stdout": self.stdout,
            "stderr": self.stderr,
            "exit_code": self.exit_code,
            "file_snapshots": self.file_snapshots,
        });
        let json =
            serde_json::to_string_pretty(&meta).map_err(|e| format!("serialize evidence: {e}"))?;
        std::fs::create_dir_all(dir).map_err(|e| format!("create evidence dir: {e}"))?;
        std::fs::write(dir.join("evidence.json"), &json)
            .map_err(|e| format!("write evidence: {e}"))?;
        for (path, content) in &self.file_snapshots {
            let target = dir.join("snapshots").join(path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("create snapshot dir: {e}"))?;
            }
            std::fs::write(&target, content).map_err(|e| format!("write snapshot {path}: {e}"))?;
        }
        Ok(())
    }
}

impl Default for Evidence {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerificationResult {
    Passed,
    Failed(String),
    Skipped(String),
}

pub trait WorldRecipe {
    fn create(&self) -> Result<World, String>;
}

#[derive(Debug)]
pub enum World {
    Filesystem(Scaffold),
    Command,
    Dolt(DoltWorld),
}

/// A language-agnostic verifier command to execute inside a [`World`].
///
/// AC tests should prefer this command boundary over Rust callbacks: projects
/// can run `cargo test`, `pytest`, `npm test`, shell scripts, or any other
/// command once a recipe has prepared the required filesystem and services.
#[derive(Debug, Clone)]
pub struct CommandRequest {
    pub command: String,
    pub cwd: Option<PathBuf>,
    pub env: HashMap<String, String>,
}

impl CommandRequest {
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
            cwd: None,
            env: HashMap::new(),
        }
    }

    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }
}

impl World {
    pub fn recipe(prefix: &str) -> E2eRecipe {
        E2eRecipe::named(prefix)
    }

    pub fn run(&self, request: CommandRequest) -> Result<Evidence, String> {
        let base_dir = self.base_dir();
        let cwd = resolve_cwd(base_dir, request.cwd.as_deref())?;
        let mut command = std::process::Command::new("sh");
        command
            .args(["-c", &request.command])
            .current_dir(cwd)
            .envs(&request.env);
        let output = command
            .output()
            .map_err(|e| format!("command failed: {e}"))?;
        Ok(Evidence::new().record_command(&output))
    }

    pub fn run_command(&self, cmd: &str) -> Result<Evidence, String> {
        self.run(CommandRequest::new(cmd))
    }

    pub fn scaffold(&self) -> Option<&Scaffold> {
        match self {
            Self::Filesystem(s) => Some(s),
            _ => None,
        }
    }

    fn base_dir(&self) -> Option<&Path> {
        match self {
            Self::Filesystem(scaffold) => Some(&scaffold.root),
            Self::Dolt(dw) => Some(dw.data_dir()),
            Self::Command => None,
        }
    }
}

fn resolve_cwd(base_dir: Option<&Path>, cwd: Option<&Path>) -> Result<PathBuf, String> {
    match (base_dir, cwd) {
        (Some(base), Some(path)) if path.is_relative() => Ok(base.join(path)),
        (_, Some(path)) => Ok(path.to_path_buf()),
        (Some(base), None) => Ok(base.to_path_buf()),
        (None, None) => std::env::current_dir().map_err(|e| format!("current dir: {e}")),
    }
}

pub struct AcTest {
    pub world: World,
    pub evidence: Evidence,
}

impl AcTest {
    pub fn new(world: World) -> Self {
        Self {
            world,
            evidence: Evidence::new(),
        }
    }

    pub fn run(&mut self, command: &str) -> Result<&mut Self, String> {
        let evidence = self.world.run_command(command)?;
        self.evidence = evidence;
        Ok(self)
    }

    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }

    pub fn pass_with(evidence: Evidence) -> VerificationResult {
        Self::evaluate(&evidence)
    }

    fn evaluate(evidence: &Evidence) -> VerificationResult {
        match evidence.exit_code {
            Some(0) => VerificationResult::Passed,
            Some(code) => {
                let msg = if evidence.stderr.is_empty() {
                    evidence.stdout.clone()
                } else {
                    format!("stderr: {}", evidence.stderr.trim())
                };
                VerificationResult::Failed(format!("exit code {code}: {msg}"))
            }
            None => VerificationResult::Skipped("No command executed".into()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn evidence_default_is_empty() {
        let e = Evidence::new();
        assert!(e.stdout.is_empty());
        assert!(e.exit_code.is_none());
    }

    #[test]
    fn evaluate_passed_on_zero_exit() {
        let e = Evidence {
            stdout: "ok".into(),
            stderr: String::new(),
            exit_code: Some(0),
            file_snapshots: HashMap::new(),
        };
        assert_eq!(AcTest::pass_with(e), VerificationResult::Passed);
    }

    #[test]
    fn evaluate_failed_on_nonzero_exit() {
        let e = Evidence {
            stdout: String::new(),
            stderr: "error".into(),
            exit_code: Some(1),
            file_snapshots: HashMap::new(),
        };
        let result = AcTest::pass_with(e);
        assert!(matches!(result, VerificationResult::Failed(_)));
    }

    #[test]
    fn evaluate_skipped_when_no_exit_code() {
        let e = Evidence::new();
        let result = AcTest::pass_with(e);
        assert_eq!(
            result,
            VerificationResult::Skipped("No command executed".into())
        );
    }

    #[test]
    fn evidence_write_to_dir_creates_json() {
        let evidence = Evidence {
            stdout: "hello".into(),
            stderr: String::new(),
            exit_code: Some(0),
            file_snapshots: HashMap::from([("config.toml".into(), "key=val".into())]),
        };
        let dir = std::env::temp_dir().join(format!(
            "evidence_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        evidence.write_to_dir(&dir).unwrap();
        assert!(dir.join("evidence.json").exists());
        assert!(dir.join("snapshots/config.toml").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn world_run_accepts_env_vars() {
        let evidence = World::Command
            .run(CommandRequest::new("printf %s \"$AC_TEST_VALUE\"").env("AC_TEST_VALUE", "ok"))
            .unwrap();
        assert_eq!(evidence.stdout, "ok");
        assert_eq!(evidence.exit_code, Some(0));
    }

    #[test]
    fn filesystem_world_run_resolves_relative_cwd_inside_scaffold() {
        let scaffold = Scaffold::new("world-cwd").unwrap();
        scaffold.write_file("nested/input.txt", "ok").unwrap();
        let evidence = World::Filesystem(scaffold)
            .run(CommandRequest::new("cat input.txt").cwd("nested"))
            .unwrap();
        assert_eq!(evidence.stdout, "ok");
        assert_eq!(evidence.exit_code, Some(0));
    }
}
