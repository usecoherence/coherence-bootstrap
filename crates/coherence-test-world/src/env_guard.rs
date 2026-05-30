use std::path::{Path, PathBuf};

pub struct EnvGuard {
    orig_dir: PathBuf,
    saved: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    pub fn save(keys: &[&str]) -> Result<Self, String> {
        let orig_dir = std::env::current_dir().map_err(|e| format!("current dir: {e}"))?;
        let saved = keys
            .iter()
            .map(|key| ((*key).to_string(), std::env::var(key).ok()))
            .collect();
        Ok(Self { orig_dir, saved })
    }

    pub fn set_current_dir(&self, path: &Path) -> Result<(), String> {
        std::env::set_current_dir(path).map_err(|e| format!("set current dir: {e}"))
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, val) in &self.saved {
            match val {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        let _ = std::env::set_current_dir(&self.orig_dir);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn env_guard_restores_env_var() {
        std::env::set_var("COHERENCE_TEST_WORLD_GUARD_TEST", "before");
        {
            let _guard = EnvGuard::save(&["COHERENCE_TEST_WORLD_GUARD_TEST"]).unwrap();
            std::env::set_var("COHERENCE_TEST_WORLD_GUARD_TEST", "during");
        }
        assert_eq!(
            std::env::var("COHERENCE_TEST_WORLD_GUARD_TEST").unwrap(),
            "before"
        );
        std::env::remove_var("COHERENCE_TEST_WORLD_GUARD_TEST");
    }

    #[test]
    fn env_guard_restores_removed_env_var() {
        std::env::remove_var("COHERENCE_TEST_WORLD_GUARD_REMOVED_TEST");
        {
            let _guard = EnvGuard::save(&["COHERENCE_TEST_WORLD_GUARD_REMOVED_TEST"]).unwrap();
            std::env::set_var("COHERENCE_TEST_WORLD_GUARD_REMOVED_TEST", "during");
        }
        assert!(std::env::var("COHERENCE_TEST_WORLD_GUARD_REMOVED_TEST").is_err());
    }
}
