use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone)]
pub struct Scaffold {
    pub root: PathBuf,
}

impl Scaffold {
    pub fn new(prefix: &str) -> Result<Self, String> {
        let root = tempfile_dir(prefix)?;
        Ok(Self { root })
    }

    pub fn write_file(&self, rel_path: &str, content: &str) -> Result<(), String> {
        let path = self.root.join(rel_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create dir {}: {e}", parent.display()))?;
        }
        fs::write(&path, content).map_err(|e| format!("write {}: {e}", path.display()))
    }

    pub fn create_dir(&self, rel_path: &str) -> Result<(), String> {
        let path = self.root.join(rel_path);
        fs::create_dir_all(&path).map_err(|e| format!("create dir {}: {e}", path.display()))
    }

    pub fn init_git(&self) -> Result<(), String> {
        let output = std::process::Command::new("git")
            .args(["init"])
            .current_dir(&self.root)
            .output()
            .map_err(|e| format!("git init: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git init failed: {stderr}"));
        }
        Ok(())
    }

    pub fn path(&self, rel_path: &str) -> PathBuf {
        self.root.join(rel_path)
    }

    pub fn exists(&self, rel_path: &str) -> bool {
        self.root.join(rel_path).exists()
    }

    pub fn read_file(&self, rel_path: &str) -> Result<String, String> {
        let path = self.root.join(rel_path);
        fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))
    }
}

impl Drop for Scaffold {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn tempfile_dir(prefix: &str) -> Result<PathBuf, String> {
    let mut path = std::env::temp_dir();
    path.push(format!("{}_{}", prefix, std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())));
    fs::create_dir_all(&path).map_err(|e| format!("create temp dir: {e}"))?;
    Ok(path)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_creates_temp_dir() {
        let s = Scaffold::new("test").unwrap();
        assert!(s.root.exists());
    }

    #[test]
    fn scaffold_writes_and_reads_file() {
        let s = Scaffold::new("test").unwrap();
        s.write_file("hello.txt", "world").unwrap();
        assert_eq!(s.read_file("hello.txt").unwrap(), "world");
    }

    #[test]
    fn scaffold_creates_nested_dirs() {
        let s = Scaffold::new("test").unwrap();
        s.write_file("a/b/c/file.txt", "content").unwrap();
        assert!(s.path("a/b/c/file.txt").exists());
    }

    #[test]
    fn scaffold_cleanup_on_drop() {
        let path;
        {
            let s = Scaffold::new("test").unwrap();
            path = s.root.clone();
            assert!(path.exists());
        }
        assert!(!path.exists());
    }
}
