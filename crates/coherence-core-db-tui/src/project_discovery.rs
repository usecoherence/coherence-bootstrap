use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn discover_projects() -> Vec<(PathBuf, String)> {
    let home = env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let mut projects = Vec::new();

    if let Ok(output) = Command::new("find")
        .args([
            format!("{}/git", home).as_str(),
            "-maxdepth",
            "6",
            "-name",
            "project.toml",
            "-path",
            "*/.coherence/project.toml",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let path = PathBuf::from(line);
            if let Some(parent) = path.parent().and_then(Path::parent) {
                let slug = parent
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                projects.push((parent.to_path_buf(), slug));
            }
        }
    }
    projects
}
