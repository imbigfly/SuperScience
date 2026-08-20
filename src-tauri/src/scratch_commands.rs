//! Startup cleanup of leftover scratch-chat sandboxes from older builds.
//! New conversations now open a normal project session.

use super::*;
use std::path::{Path, PathBuf};

fn scratch_sandbox_root(app_data: &Path) -> PathBuf {
    app_data.join("scratch")
}

/// Scratch projects and sandbox directories a previous run left behind.
#[derive(Default)]
pub(super) struct OrphanScratchProjects {
    projects: Vec<(String, String)>,
    sandboxes: Vec<PathBuf>,
}

/// Name the orphans while nothing else can create a scratch chat yet. Deleting
/// a sandbox walks a whole directory tree, so the deletion itself runs later
/// (see `purge_orphan_scratch_projects`) and must not see sandboxes created
/// after startup.
pub(super) async fn collect_orphan_scratch_projects(
    store: &Store,
    app_data: &Path,
) -> OrphanScratchProjects {
    let projects = store.list_scratch_projects().await.unwrap_or_default();
    let sandboxes = std::fs::read_dir(scratch_sandbox_root(app_data))
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|entry| entry.path())
                .collect()
        })
        .unwrap_or_default();
    OrphanScratchProjects {
        projects,
        sandboxes,
    }
}

pub(super) async fn purge_orphan_scratch_projects(store: &Store, orphans: OrphanScratchProjects) {
    for (id, ws) in orphans.projects {
        let _ = store.delete_project(&id).await;
        if !ws.is_empty() {
            let _ = std::fs::remove_dir_all(&ws);
        }
    }
    for sandbox in orphans.sandboxes {
        let _ = std::fs::remove_dir_all(sandbox);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use superscience_store::{Store, SCRATCH_PROJECT_PREFIX};

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

        let orphans = collect_orphan_scratch_projects(&store, &app_data).await;
        purge_orphan_scratch_projects(&store, orphans).await;

        assert!(store.get_project(&project_id).await.unwrap().is_none());
        assert!(!orphan_dir.exists());
        let _ = std::fs::remove_dir_all(&app_data);
    }

    #[tokio::test]
    async fn purge_spares_scratch_chats_created_after_startup() {
        let app_data = std::env::temp_dir().join(format!("wisp_scratch_race_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&app_data).unwrap();
        let store = Store::open(&app_data.join("superscience.sqlite"))
            .await
            .unwrap();
        let orphan_dir = scratch_sandbox_root(&app_data).join("orphan");
        std::fs::create_dir_all(&orphan_dir).unwrap();
        let orphan_id = format!("{SCRATCH_PROJECT_PREFIX}orphan");
        store
            .create_project(&orphan_id, "Scratch", &orphan_dir.to_string_lossy())
            .await
            .unwrap();

        let orphans = collect_orphan_scratch_projects(&store, &app_data).await;

        // A directory created after the scan must survive the deferred purge.
        let fresh_dir = scratch_sandbox_root(&app_data).join("fresh");
        std::fs::create_dir_all(&fresh_dir).unwrap();
        let fresh_id = format!("{SCRATCH_PROJECT_PREFIX}fresh");
        store
            .create_project(&fresh_id, "Scratch", &fresh_dir.to_string_lossy())
            .await
            .unwrap();

        purge_orphan_scratch_projects(&store, orphans).await;

        assert!(store.get_project(&orphan_id).await.unwrap().is_none());
        assert!(!orphan_dir.exists());
        assert!(store.get_project(&fresh_id).await.unwrap().is_some());
        assert!(fresh_dir.exists());
        let _ = std::fs::remove_dir_all(&app_data);
    }
}
