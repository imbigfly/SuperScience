//! Ephemeral scratch chat: temp sandbox workspace, no project binding, destroyed on close.

use super::*;
use crate::project_commands::{cancel_project_sessions, load_active_project, set_active_project};
use std::path::{Path, PathBuf};
use superscience_store::SCRATCH_PROJECT_PREFIX;

pub(super) fn is_scratch_project_id(id: &str) -> bool {
    superscience_store::is_scratch_project_id(id)
}

#[derive(Clone)]
pub(super) struct ScratchWindow {
    prev_project_id: String,
    scratch_project_id: String,
    session_id: String,
    sandbox_path: PathBuf,
}

fn scratch_sandbox_root(app_data: &Path) -> PathBuf {
    app_data.join("scratch")
}

pub(super) async fn purge_orphan_scratch_projects(store: &Store, app_data: &Path) {
    let scratch_root = scratch_sandbox_root(app_data);
    let rows = store.list_scratch_projects().await.unwrap_or_default();
    for (id, ws) in rows {
        let _ = store.delete_project(&id).await;
        if !ws.is_empty() {
            let _ = std::fs::remove_dir_all(&ws);
        }
    }
    if let Ok(entries) = std::fs::read_dir(&scratch_root) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
}

async fn destroy_scratch_window(state: &AppState, label: &str, scratch: ScratchWindow) {
    cancel_project_sessions(state, &scratch.scratch_project_id).await;
    state
        .runtime_manager
        .stop_project(&scratch.scratch_project_id)
        .await;
    let _ = state
        .store
        .delete_project(&scratch.scratch_project_id)
        .await;
    let _ = std::fs::remove_dir_all(&scratch.sandbox_path);
    if state.active_frame(label).as_deref() == Some(scratch.session_id.as_str()) {
        state.set_active_frame(label, None);
    }
    let _ = set_active_project(state, label, &scratch.prev_project_id).await;
}

async fn close_scratch_for_window(state: &AppState, label: &str) {
    let scratch = state.scratch.write().unwrap().remove(label);
    if let Some(scratch) = scratch {
        destroy_scratch_window(state, label, scratch).await;
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ScratchChatInfo {
    session_id: String,
    project_id: String,
}

#[tauri::command]
pub(super) async fn start_scratch_chat(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<ScratchChatInfo, String> {
    let label = window.label().to_string();
    close_scratch_for_window(state.inner(), &label).await;

    let prev = state.active(&label);
    if is_scratch_project_id(&prev.id) {
        return Err("Scratch chat is already active.".into());
    }

    let uuid = Uuid::new_v4().to_string();
    let sandbox_path = scratch_sandbox_root(&state.app_data).join(&uuid);
    std::fs::create_dir_all(&sandbox_path)
        .map_err(|e| format!("Failed to create scratch sandbox: {e}"))?;
    let marker = sandbox_path.join(".superscience-write-test");
    std::fs::write(&marker, b"").map_err(|e| format!("Scratch sandbox is not writable: {e}"))?;
    let _ = std::fs::remove_file(&marker);

    let project_id = format!("{SCRATCH_PROJECT_PREFIX}{uuid}");
    let ws = sandbox_path.to_string_lossy().into_owned();
    state
        .store
        .create_project(&project_id, "Scratch", &ws)
        .await
        .map_err(|e| format!("{e}"))?;

    let session_id = create_session_frame(&state.store, &project_id).await?;
    let (ap, _, _) = load_active_project(state.inner(), &project_id).await?;
    state.set_active(&label, ap);
    state.set_active_frame(&label, Some(session_id.clone()));
    state.scratch.write().unwrap().insert(
        label,
        ScratchWindow {
            prev_project_id: prev.id,
            scratch_project_id: project_id.clone(),
            session_id: session_id.clone(),
            sandbox_path,
        },
    );

    Ok(ScratchChatInfo {
        session_id,
        project_id,
    })
}

#[tauri::command]
pub(super) async fn close_scratch_chat(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    close_scratch_for_window(state.inner(), window.label()).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use superscience_store::Store;

    #[tokio::test]
    async fn purge_orphan_scratch_projects_cleans_db_and_dirs() {
        let app_data =
            std::env::temp_dir().join(format!("superscience_scratch_purge_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&app_data).unwrap();
        let db = app_data.join("superscience.sqlite");
        let store = Store::open(&db).await.unwrap();
        let orphan_dir = scratch_sandbox_root(&app_data).join("orphan");
        std::fs::create_dir_all(&orphan_dir).unwrap();
        let project_id = format!("{SCRATCH_PROJECT_PREFIX}orphan");
        store
            .create_project(&project_id, "Scratch", &orphan_dir.to_string_lossy())
            .await
            .unwrap();

        purge_orphan_scratch_projects(&store, &app_data).await;

        assert!(store.get_project(&project_id).await.unwrap().is_none());
        assert!(!orphan_dir.exists());
        let _ = std::fs::remove_dir_all(&app_data);
    }
}
