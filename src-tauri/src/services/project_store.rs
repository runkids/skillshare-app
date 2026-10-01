use crate::models::project::{Project, ProjectStore, ProjectType};
use std::path::{Path, PathBuf};

fn store_path() -> PathBuf {
    crate::utils::paths::app_data_dir().join("projects.json")
}

pub fn load() -> ProjectStore {
    load_from(&store_path())
}

/// Load the store, moving an unreadable file aside so the next save
/// cannot overwrite the user's project list with an empty one.
fn load_from(path: &Path) -> ProjectStore {
    if !path.exists() {
        return ProjectStore::default();
    }
    let parsed = std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|data| serde_json::from_slice(&data).map_err(|e| e.to_string()));
    match parsed {
        Ok(store) => store,
        Err(e) => {
            let ts = chrono::Utc::now().timestamp();
            let backup = path.with_file_name(format!("projects.json.corrupt-{ts}"));
            match std::fs::rename(path, &backup) {
                Ok(()) => log::warn!(
                    "Could not read {} ({e}); moved it to {}",
                    path.display(),
                    backup.display()
                ),
                Err(re) => log::warn!(
                    "Could not read {} ({e}) or move it aside ({re})",
                    path.display()
                ),
            }
            ProjectStore::default()
        }
    }
}

pub fn save(store: &ProjectStore) -> Result<(), String> {
    save_to(&store_path(), store)
}

fn save_to(path: &Path, store: &ProjectStore) -> Result<(), String> {
    let data = serde_json::to_string_pretty(store).map_err(|e| format!("Serialize error: {e}"))?;
    crate::utils::fs::write_atomic(path, data.as_bytes()).map_err(|e| format!("Write error: {e}"))
}

/// Canonical form of a project path for duplicate checks.
fn normalize_path(path: &str) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| {
        let trimmed = path.trim_end_matches('/');
        PathBuf::from(if trimmed.is_empty() { "/" } else { trimmed })
    })
}

pub fn add_project(
    store: &mut ProjectStore,
    name: String,
    path: String,
    project_type: ProjectType,
) -> Result<Project, String> {
    let normalized = normalize_path(&path);
    if store
        .projects
        .iter()
        .any(|p| normalize_path(&p.path) == normalized)
    {
        return Err(format!("{path} is already in your projects"));
    }
    let project = Project {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        path,
        project_type,
        added_at: chrono::Utc::now().to_rfc3339(),
    };
    store.projects.push(project.clone());
    if store.active_project_id.is_none() {
        store.active_project_id = Some(project.id.clone());
    }
    Ok(project)
}

pub fn remove_project(store: &mut ProjectStore, id: &str) {
    store.projects.retain(|p| p.id != id);
    if store.active_project_id.as_deref() == Some(id) {
        store.active_project_id = store.projects.first().map(|p| p.id.clone());
    }
}

/// Returns (project_dir, is_project_mode) for the active project.
/// Used by server start/restart to determine CLI flags.
pub fn active_project_mode(store: &ProjectStore) -> (Option<String>, bool) {
    let active = store.active_project();
    let dir = active.map(|p| p.path.clone());
    let is_project = active
        .map(|p| p.project_type == ProjectType::Project)
        .unwrap_or(false);
    (dir, is_project)
}

pub fn set_active(store: &mut ProjectStore, id: &str) -> Result<(), String> {
    if store.projects.iter().any(|p| p.id == id) {
        store.active_project_id = Some(id.to_string());
        Ok(())
    } else {
        Err(format!("Project {id} not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ss-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok();
        dir
    }

    fn store_with(path: &str) -> ProjectStore {
        let mut store = ProjectStore::default();
        add_project(&mut store, "a".into(), path.into(), ProjectType::Project).ok();
        store
    }

    #[test]
    fn corrupt_file_is_backed_up_before_the_next_save() {
        let dir = sandbox("corrupt");
        let path = dir.join("projects.json");
        std::fs::write(&path, "{ not json").ok();

        let store = load_from(&path);
        save_to(&path, &store).ok();

        let backups: Vec<String> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("projects.json.corrupt-"))
            .filter_map(|n| std::fs::read_to_string(dir.join(n)).ok())
            .collect();
        assert_eq!(backups, vec!["{ not json".to_string()]);
    }

    #[test]
    fn saved_store_reads_back_as_valid_json() {
        let path = sandbox("save").join("projects.json");
        save_to(&path, &store_with("/tmp/repo")).ok();

        let data = std::fs::read_to_string(&path).unwrap_or_default();
        let parsed = serde_json::from_str::<ProjectStore>(&data).ok();
        let first = parsed.and_then(|s| s.projects.first().map(|p| p.path.clone()));
        assert_eq!(first.as_deref(), Some("/tmp/repo"));
    }

    #[test]
    fn rejects_a_path_that_is_already_added() {
        let mut store = store_with("/nonexistent/ss-repo");
        let added = add_project(
            &mut store,
            "b".into(),
            "/nonexistent/ss-repo".into(),
            ProjectType::Project,
        );
        assert!(added.is_err());
    }

    #[test]
    fn rejects_the_same_path_with_a_trailing_slash() {
        let path = sandbox("dup").to_string_lossy().into_owned();
        let mut store = store_with(&path);
        let added = add_project(
            &mut store,
            "b".into(),
            format!("{path}/"),
            ProjectType::Project,
        );
        assert_eq!(
            added.err(),
            Some(format!("{path}/ is already in your projects"))
        );
    }
}
