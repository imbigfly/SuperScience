//! Immutable project state captured after each completed native mainline turn.
//!
//! The visual `User` event count supplies a stable turn index because model
//! context compaction may rewrite message sequence numbers. Workspace bytes are
//! stored by the exploration snapshot backend, which deduplicates blobs by
//! SHA-256; revision rows only retain immutable manifests and state membership.

use crate::exploration_commands::write_context_archive;
use crate::exploration_workspace::{ExplorationWorkspaceBackend, PersistentExplorationWorkspace};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use wisp_store::{
    ContextArchiveRecord, ProjectStateRevision, StateScope, Store, WorkspaceSnapshotRecord,
    MAINLINE_SCOPE_KEY,
};

const FULL_REVISION_INTERVAL: i64 = 10;

pub(crate) async fn record_completed_mainline_turn(
    store: &Store,
    app_data: &Path,
    project_id: &str,
    frame_id: &str,
    project_root: &Path,
) -> Result<Option<ProjectStateRevision>, String> {
    let scope = store
        .frame_state_scope(frame_id)
        .await
        .map_err(|error| error.to_string())?;
    if !matches!(scope, Some(StateScope::Mainline { project_id: ref owner }) if owner == project_id)
    {
        return Ok(None);
    }

    let messages = store
        .load_messages_with_seq(frame_id)
        .await
        .map_err(|error| error.to_string())?;
    let Some((message_seq, last)) = messages.last() else {
        return Ok(None);
    };
    if last.role != wisp_llm::Role::Assistant || !last.tool_calls.is_empty() {
        return Ok(None);
    }
    let visual_turn_count = store
        .frame_visual_user_turn_count(frame_id)
        .await
        .map_err(|error| error.to_string())?;
    if visual_turn_count <= 0 {
        return Ok(None);
    }
    let turn_index = visual_turn_count - 1;
    if let Some(existing) = store
        .project_state_revision_for_turn(frame_id, turn_index)
        .await
        .map_err(|error| error.to_string())?
    {
        return Ok(Some(existing));
    }

    let project_root = dunce::canonicalize(project_root)
        .map_err(|error| format!("cannot resolve project workspace: {error}"))?;
    let backend = PersistentExplorationWorkspace::new(app_data.to_path_buf());
    let snapshot = backend.checkpoint(&project_root).await?;
    store
        .create_workspace_snapshot(&WorkspaceSnapshotRecord {
            id: snapshot.id.clone(),
            project_id: project_id.to_string(),
            manifest_json: serde_json::to_string(&snapshot).map_err(|error| error.to_string())?,
            manifest_sha256: snapshot.manifest_sha256.clone(),
            created_at: snapshot.created_at,
        })
        .await
        .map_err(|error| error.to_string())?;

    let parent = store
        .latest_project_state_revision(frame_id)
        .await
        .map_err(|error| error.to_string())?;
    let is_full = parent.is_none() || turn_index % FULL_REVISION_INTERVAL == 0;
    let workspace_delta_json = if is_full {
        serde_json::json!({
            "kind": "full",
            "entries": &snapshot.entries,
            "warnings": &snapshot.warnings,
        })
        .to_string()
    } else {
        let base = backend.load_snapshot(
            &parent
                .as_ref()
                .expect("non-full revisions have a parent")
                .workspace_snapshot_id,
        )?;
        let delta = backend.diff(&base, &project_root).await?;
        serde_json::json!({ "kind": "delta", "files": delta }).to_string()
    };

    let now = chrono::Utc::now().timestamp();
    let archive_id = uuid::Uuid::new_v4().to_string();
    let archive_relative = PathBuf::from("exploration-contexts").join(format!("{archive_id}.json"));
    let archive_path = app_data.join(&archive_relative);
    let archived_messages = messages
        .iter()
        .map(|(_, message)| message.clone())
        .collect::<Vec<_>>();
    write_context_archive(
        app_data,
        &archive_path,
        frame_id,
        *message_seq,
        &archived_messages,
    )?;
    let archive_bytes = std::fs::read(&archive_path).map_err(|error| error.to_string())?;
    store
        .create_context_archive(&ContextArchiveRecord {
            id: archive_id.clone(),
            project_id: project_id.to_string(),
            frame_id: frame_id.to_string(),
            storage_path: archive_relative.to_string_lossy().replace('\\', "/"),
            checksum: hex::encode(Sha256::digest(&archive_bytes)),
            created_at: now,
        })
        .await
        .map_err(|error| error.to_string())?;

    let artifact_heads = store
        .list_artifact_heads(project_id, MAINLINE_SCOPE_KEY)
        .await
        .map_err(|error| error.to_string())?;
    let entities = store
        .snapshot_mainline_entities(project_id)
        .await
        .map_err(|error| error.to_string())?;
    let run_ids = entities
        .iter()
        .filter(|entity| entity.entity_kind == "run")
        .map(|entity| entity.entity_id.clone())
        .collect::<Vec<_>>();
    let decision_ids = store
        .list_mainline_decision_ids(project_id)
        .await
        .map_err(|error| error.to_string())?;
    let external_resources = entities
        .iter()
        .filter(|entity| entity.entity_kind == "external_resource")
        .cloned()
        .collect::<Vec<_>>();
    let state_generation = store
        .project_state_generation(project_id)
        .await
        .map_err(|error| error.to_string())?;
    let ui_event_seq = store
        .frame_ui_event_head(frame_id)
        .await
        .map_err(|error| error.to_string())?;
    let revision = ProjectStateRevision {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project_id.to_string(),
        frame_id: frame_id.to_string(),
        turn_index,
        message_seq: *message_seq,
        ui_event_seq,
        parent_revision_id: parent.map(|revision| revision.id),
        workspace_snapshot_id: snapshot.id,
        workspace_manifest_sha256: snapshot.manifest_sha256,
        workspace_delta_json,
        artifact_heads_json: serde_json::to_string(&artifact_heads)
            .map_err(|error| error.to_string())?,
        entities_json: serde_json::to_string(&entities).map_err(|error| error.to_string())?,
        run_ids_json: serde_json::to_string(&run_ids).map_err(|error| error.to_string())?,
        decision_ids_json: serde_json::to_string(&decision_ids)
            .map_err(|error| error.to_string())?,
        external_effects_json: serde_json::json!({
            "recoverability": "metadata_only",
            "resources": external_resources,
        })
        .to_string(),
        context_archive_id: archive_id,
        state_generation,
        is_full,
        created_at: now,
    };
    if store
        .create_project_state_revision(&revision)
        .await
        .map_err(|error| error.to_string())?
    {
        Ok(Some(revision))
    } else {
        store
            .project_state_revision_for_turn(frame_id, turn_index)
            .await
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exploration_commands::ExplorationService;

    async fn fixture(label: &str) -> (Store, PathBuf, PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "wisp_project_state_revision_{label}_{}",
            uuid::Uuid::new_v4()
        ));
        let project = base.join("project");
        let app_data = base.join("app-data");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&app_data).unwrap();
        let store = Store::open(&base.join("store.sqlite")).await.unwrap();
        store
            .create_project("p", "Project", &project.to_string_lossy())
            .await
            .unwrap();
        store
            .create_frame("main", "p", "OPERON", "model")
            .await
            .unwrap();
        (store, base, project, app_data)
    }

    async fn append_turn(store: &Store, index: i64, question: &str, answer: &str) {
        let message_seq = index * 2 + 1;
        store
            .append_message("main", message_seq, &wisp_llm::Message::user(question))
            .await
            .unwrap();
        store
            .append_message(
                "main",
                message_seq + 1,
                &wisp_llm::Message::assistant(answer),
            )
            .await
            .unwrap();
        store
            .append_session_ui_event(
                "main",
                index + 1,
                &serde_json::json!({
                    "kind": "User",
                    "frame_id": "main",
                    "text": question,
                })
                .to_string(),
            )
            .await
            .unwrap();
    }

    fn file_count(root: &Path) -> usize {
        let Ok(entries) = std::fs::read_dir(root) else {
            return 0;
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| {
                if entry.path().is_dir() {
                    file_count(&entry.path())
                } else {
                    1
                }
            })
            .sum()
    }

    #[tokio::test]
    async fn historical_exploration_restores_second_turn_after_compaction() {
        let (store, base, project, app_data) = fixture("historical").await;
        std::fs::write(project.join("shared.txt"), b"unchanged").unwrap();

        append_turn(&store, 0, "first question", "first answer").await;
        std::fs::write(project.join("state.txt"), b"version one").unwrap();
        let first = record_completed_mainline_turn(&store, &app_data, "p", "main", &project)
            .await
            .unwrap()
            .unwrap();

        append_turn(&store, 1, "second question", "second answer").await;
        // This simulates an editor write that did not pass through a Wisp tool.
        std::fs::write(project.join("state.txt"), b"version two from editor").unwrap();
        let second = record_completed_mainline_turn(&store, &app_data, "p", "main", &project)
            .await
            .unwrap()
            .unwrap();

        append_turn(&store, 2, "third question", "third answer").await;
        std::fs::write(project.join("state.txt"), b"version three").unwrap();
        let third = record_completed_mainline_turn(&store, &app_data, "p", "main", &project)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            (first.turn_index, second.turn_index, third.turn_index),
            (0, 1, 2)
        );
        assert!(first.is_full);
        assert!(!second.is_full);
        let delta: serde_json::Value = serde_json::from_str(&second.workspace_delta_json).unwrap();
        assert_eq!(delta["kind"], "delta");
        assert!(delta["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["path"] == "state.txt" && file["kind"] == "modified"));
        assert_eq!(
            store
                .list_project_state_revisions("main")
                .await
                .unwrap()
                .len(),
            3
        );

        let blob_root = app_data.join("exploration-snapshots/blobs/sha256");
        assert_eq!(
            file_count(&blob_root),
            4,
            "unchanged content reuses its blob"
        );

        // Rewrite the live model context as compaction would. Stable visual
        // indices and immutable context archives must still reconstruct turn 2.
        store
            .replace_messages(
                "main",
                &[
                    wisp_llm::Message::user("compacted context"),
                    wisp_llm::Message::assistant("third answer"),
                ],
            )
            .await
            .unwrap();
        assert_eq!(store.frame_visual_user_turn_count("main").await.unwrap(), 3);

        let service = ExplorationService::new(store.clone(), app_data.clone());
        let checkpoint = service
            .create_checkpoint_at("p", "main", Some(1))
            .await
            .unwrap();
        assert_eq!(
            checkpoint.workspace_snapshot_id,
            second.workspace_snapshot_id
        );
        let exploration = service
            .create_exploration(&checkpoint.id, "Second turn")
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(Path::new(&exploration.workspace_dir).join("state.txt")).unwrap(),
            b"version two from editor"
        );
        let cloned = store.load_messages(&exploration.frame_id).await.unwrap();
        assert_eq!(cloned.len(), 4);
        assert_eq!(cloned.last().unwrap().content.as_text(), "second answer");
        assert!(!cloned
            .iter()
            .any(|message| message.content.as_text().contains("third question")));

        drop(service);
        drop(store);
        let _ = std::fs::remove_dir_all(base);
    }

    #[tokio::test]
    async fn upgrade_history_is_unavailable_but_latest_turn_falls_back() {
        let (store, base, project, app_data) = fixture("legacy").await;
        append_turn(&store, 0, "legacy one", "legacy answer one").await;
        append_turn(&store, 1, "legacy two", "legacy answer two").await;
        append_turn(&store, 2, "current", "current answer").await;
        std::fs::write(project.join("state.txt"), b"current").unwrap();
        let service = ExplorationService::new(store.clone(), app_data);

        let error = service
            .create_checkpoint_at("p", "main", Some(1))
            .await
            .unwrap_err();
        assert!(error.starts_with("exploration_history_unavailable"));
        assert!(service
            .create_checkpoint_at("p", "main", Some(2))
            .await
            .is_ok());

        drop(service);
        drop(store);
        let _ = std::fs::remove_dir_all(base);
    }
}
