use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn discover_projects() -> Vec<(PathBuf, String)> {
    let home = env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let mut projects = Vec::new();

    if let Some(current) = current_project_root() {
        push_project(&mut projects, current);
    }

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
                push_project(&mut projects, parent.to_path_buf());
            }
        }
    }
    projects
}

fn current_project_root() -> Option<PathBuf> {
    let mut cur = env::current_dir().ok()?;
    loop {
        if cur.join(".coherence/project.toml").is_file() {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}

fn push_project(projects: &mut Vec<(PathBuf, String)>, path: PathBuf) {
    if projects.iter().any(|(existing, _)| existing == &path) {
        return;
    }
    let slug = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    projects.push((path, slug));
}
