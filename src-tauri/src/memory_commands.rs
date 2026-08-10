//! Memory Commands split out of lib.rs; shared state/helpers stay in the crate root.

use super::*;

#[derive(Serialize, Clone)]
pub(super) struct MemoryView {
    enabled: bool,
    project_id: String,
    project_name: String,
    today_file: String,
    files: Vec<MemoryFile>,
}

/// Resolve which project's `.superscience/memory` to open. `None` / empty uses the
/// window's active project; otherwise load that project's workspace without
/// switching the active chat project.
async fn resolve_memory_target(
    state: &AppState,
    window_label: &str,
    project_id: Option<String>,
) -> Result<(String, String, MemoryManager), String> {
    let ap = state.active(window_label);
    let requested = project_id
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty());
    let id = requested.unwrap_or_else(|| ap.id.clone());

    let (name, root) = if id == ap.id {
        let name = state
            .store
            .get_project(&ap.id)
            .await
            .ok()
            .flatten()
            .map(|(name, _)| name)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| ap.id.clone());
        (name, ap.root.clone())
    } else {
        let (name, workspace) = state
            .store
            .get_project(&id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("project not found: {id}"))?;
        let workspace = workspace.trim().to_string();
        if workspace.is_empty() {
            return Err(format!("project {id} has no workspace directory"));
        }
        let name = if name.trim().is_empty() {
            id.clone()
        } else {
            name
        };
        (name, PathBuf::from(workspace))
    };

    Ok((id, name, MemoryManager::at(&root)))
}

async fn build_memory_view(
    state: &AppState,
    window_label: &str,
    enabled: bool,
    project_id: Option<String>,
) -> Result<MemoryView, String> {
    let (project_id, project_name, memory) =
        resolve_memory_target(state, window_label, project_id).await?;
    Ok(MemoryView {
        enabled,
        project_id,
        project_name,
        today_file: chrono::Local::now().format("%Y-%m-%d.md").to_string(),
        files: list_memory_files(&memory),
    })
}

async fn require_writable_memory_target(
    state: &AppState,
    window_label: &str,
    project_id: &str,
) -> Result<wisp_store::StateScope, String> {
    let (_, active_scope) =
        exploration_commands::working_project_for_active_frame(state, window_label).await?;
    let scope = if active_scope.project_id() == project_id {
        active_scope
    } else {
        wisp_store::StateScope::mainline(project_id.to_string())
    };
    exploration_commands::require_writable_scope(&state.store, &scope).await?;
    Ok(scope)
}

#[tauri::command]
pub(super) async fn get_memory_view(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    project_id: Option<String>,
) -> Result<MemoryView, String> {
    let enabled = load_memory_enabled(&state.store).await;
    build_memory_view(&state, window.label(), enabled, project_id).await
}

#[tauri::command]
pub(super) async fn set_memory_enabled(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    enabled: bool,
    project_id: Option<String>,
) -> Result<MemoryView, String> {
    save_memory_enabled(&state.store, enabled).await?;
    clear_idle_agents(&state).await;
    build_memory_view(&state, window.label(), enabled, project_id).await
}

#[tauri::command]
pub(super) async fn read_memory_file(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    name: String,
    project_id: Option<String>,
) -> Result<String, String> {
    let (_, _, memory) = resolve_memory_target(&state, window.label(), project_id).await?;
    let path = memory_file_path(&memory, &name)?;
    if !path.is_file() {
        return Ok(String::new());
    }
    std::fs::read_to_string(&path).map_err(|e| format!("{e}"))
}

#[tauri::command]
pub(super) async fn write_memory_file(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    name: String,
    content: String,
    project_id: Option<String>,
) -> Result<Vec<MemoryFile>, String> {
    let target_project_id = project_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| state.active(window.label()).id);
    let _project_activity = state.begin_project_activity(&target_project_id)?;
    let (id, _, memory) = resolve_memory_target(&state, window.label(), project_id).await?;
    let scope = require_writable_memory_target(&state, window.label(), &id).await?;
    let path = memory_file_path(&memory, &name)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{e}"))?;
    }
    std::fs::write(&path, content).map_err(|e| format!("{e}"))?;
    state
        .store
        .bump_state_generation(&scope)
        .await
        .map_err(|error| error.to_string())?;
    Ok(list_memory_files(&memory))
}

#[tauri::command]
pub(super) async fn delete_memory_file(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    name: String,
    project_id: Option<String>,
) -> Result<Vec<MemoryFile>, String> {
    let target_project_id = project_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| state.active(window.label()).id);
    let _project_activity = state.begin_project_activity(&target_project_id)?;
    let (id, _, memory) = resolve_memory_target(&state, window.label(), project_id).await?;
    let scope = require_writable_memory_target(&state, window.label(), &id).await?;
    let path = memory_file_path(&memory, &name)?;
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|e| format!("{e}"))?;
        state
            .store
            .bump_state_generation(&scope)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(list_memory_files(&memory))
}

#[tauri::command]
pub(super) async fn clear_memory(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    project_id: Option<String>,
) -> Result<Vec<MemoryFile>, String> {
    let target_project_id = project_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| state.active(window.label()).id);
    let _project_activity = state.begin_project_activity(&target_project_id)?;
    let (id, _, memory) = resolve_memory_target(&state, window.label(), project_id).await?;
    let scope = require_writable_memory_target(&state, window.label(), &id).await?;
    let Ok(rd) = std::fs::read_dir(memory.dir()) else {
        return Ok(vec![]);
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let _ = std::fs::remove_file(path);
        }
    }
    state
        .store
        .bump_state_generation(&scope)
        .await
        .map_err(|error| error.to_string())?;
    Ok(list_memory_files(&memory))
}
