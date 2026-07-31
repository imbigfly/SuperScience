use super::{
    artifact_node_id, artifact_version_from_row, session_display_title, ArtifactCaptureTiming,
    ArtifactMaterialization, ArtifactSearchResult, ArtifactVersion, ArtifactVersionDraft,
    ResearchNode, ResearchNodeKind, Store,
};
use anyhow::Result;
use sha2::{Digest, Sha256};
use sqlx::Row;

pub fn logical_artifact_id(project_id: &str, logical_key: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(project_id.as_bytes());
    digest.update([0]);
    digest.update(logical_key.as_bytes());
    format!("artifact-{}", &hex::encode(digest.finalize())[..32])
}

impl Store {
    pub async fn save_artifact(
        &self,
        id: &str,
        project_id: &str,
        root_frame_id: &str,
        filename: &str,
        content_type: &str,
        storage_path: &str,
    ) -> Result<String> {
        self.save_artifact_version(&ArtifactVersionDraft {
            version_id: None,
            artifact_id: id.to_string(),
            project_id: project_id.to_string(),
            root_frame_id: root_frame_id.to_string(),
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            storage_path: storage_path.to_string(),
            logical_key: None,
            size_bytes: None,
            checksum: None,
            producing_run_id: None,
            env_snapshot_hash: None,
            materialization: ArtifactMaterialization::Reference,
            capture_timing: ArtifactCaptureTiming::Unknown,
        })
        .await
    }

    pub async fn save_artifact_version(&self, draft: &ArtifactVersionDraft) -> Result<String> {
        if draft.artifact_id.trim().is_empty()
            || draft.project_id.trim().is_empty()
            || draft.root_frame_id.trim().is_empty()
            || draft.filename.trim().is_empty()
            || draft.storage_path.trim().is_empty()
        {
            anyhow::bail!("Artifact version requires identity, project, frame, name, and storage");
        }
        if draft
            .logical_key
            .as_deref()
            .is_some_and(|key| key.trim().is_empty())
        {
            anyhow::bail!("Artifact logical_key cannot be empty");
        }
        if draft.checksum.as_deref().is_some_and(|checksum| {
            checksum.len() != 64 || !checksum.bytes().all(|b| b.is_ascii_hexdigit())
        }) {
            anyhow::bail!("Artifact checksum must be a SHA-256 hex digest");
        }

        let now = chrono::Utc::now().timestamp();
        let mut tx = self.begin_write().await?;
        let frame_project: Option<String> =
            sqlx::query_scalar("SELECT project_id FROM frames WHERE id=?")
                .bind(&draft.root_frame_id)
                .fetch_optional(&mut *tx)
                .await?;
        if frame_project.as_deref() != Some(draft.project_id.as_str()) {
            anyhow::bail!("Artifact source frame must belong to the Artifact project");
        }
        if let Some(run_id) = draft.producing_run_id.as_deref() {
            let run_project: Option<String> =
                sqlx::query_scalar("SELECT project_id FROM runs WHERE id=?")
                    .bind(run_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            if run_project.as_deref() != Some(draft.project_id.as_str()) {
                anyhow::bail!("Producing Run must belong to the Artifact project");
            }
        }
        if let Some(env_hash) = draft.env_snapshot_hash.as_deref() {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM env_snapshots WHERE hash=?)")
                    .bind(env_hash)
                    .fetch_one(&mut *tx)
                    .await?;
            if !exists {
                anyhow::bail!("Artifact environment snapshot does not exist");
            }
        }
        let existing: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT project_id,logical_key FROM artifacts WHERE id=?")
                .bind(&draft.artifact_id)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some((project_id, logical_key)) = existing {
            if project_id != draft.project_id {
                anyhow::bail!("Artifact cannot move between projects");
            }
            if let (Some(existing), Some(requested)) =
                (logical_key.as_deref(), draft.logical_key.as_deref())
            {
                if existing != requested {
                    anyhow::bail!("Artifact logical identity cannot be changed");
                }
            }
        }
        let parent_version_id: Option<String> =
            sqlx::query_scalar("SELECT latest_version_id FROM artifacts WHERE id=?")
                .bind(&draft.artifact_id)
                .fetch_optional(&mut *tx)
                .await?
                .flatten();
        let version_number: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_number), 0) + 1 FROM artifact_versions WHERE artifact_id=?",
        )
        .bind(&draft.artifact_id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO artifacts(\
                id,project_id,root_frame_id,filename,content_type,storage_path,created_at,\
                latest_version_id,logical_key\
             ) VALUES(?,?,?,?,?,?,?,NULL,?) \
             ON CONFLICT(id) DO UPDATE SET \
                filename=excluded.filename,content_type=excluded.content_type,\
                storage_path=excluded.storage_path,\
                logical_key=COALESCE(artifacts.logical_key,excluded.logical_key)",
        )
        .bind(&draft.artifact_id)
        .bind(&draft.project_id)
        .bind(&draft.root_frame_id)
        .bind(&draft.filename)
        .bind(&draft.content_type)
        .bind(&draft.storage_path)
        .bind(now)
        .bind(draft.logical_key.as_deref())
        .execute(&mut *tx)
        .await?;
        let version_id = draft
            .version_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        if version_id.trim().is_empty() {
            anyhow::bail!("Artifact version id cannot be empty");
        }
        sqlx::query(
            "INSERT INTO artifact_versions(\
                id,artifact_id,version_number,content_type,storage_path,size_bytes,checksum,\
                parent_version_id,producing_run_id,env_snapshot_hash,materialization,\
                capture_timing,created_at\
             ) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(&version_id)
        .bind(&draft.artifact_id)
        .bind(version_number)
        .bind(&draft.content_type)
        .bind(&draft.storage_path)
        .bind(draft.size_bytes)
        .bind(draft.checksum.as_deref())
        .bind(parent_version_id)
        .bind(draft.producing_run_id.as_deref())
        .bind(draft.env_snapshot_hash.as_deref())
        .bind(draft.materialization.as_str())
        .bind(draft.capture_timing.as_str())
        .bind(now)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE artifacts SET latest_version_id=? WHERE id=?")
            .bind(&version_id)
            .bind(&draft.artifact_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        let mut node = ResearchNode::new(
            artifact_node_id(&draft.artifact_id),
            &draft.project_id,
            ResearchNodeKind::Artifact,
            &draft.filename,
        )?;
        node.ref_id = Some(draft.artifact_id.clone());
        self.save_research_node(&node).await?;
        Ok(version_id)
    }

    /// Relocate the current storage target without creating a new scientific
    /// version. This is used when an isolated Agent workspace is removed after
    /// its immutable artifact bytes have been copied to durable app storage.
    pub async fn relocate_artifact_storage(&self, id: &str, storage_path: &str) -> Result<bool> {
        let mut tx = self.begin_write().await?;
        let latest_version_id: Option<String> =
            sqlx::query_scalar("SELECT latest_version_id FROM artifacts WHERE id=?")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
                .flatten();
        let updated = sqlx::query("UPDATE artifacts SET storage_path=? WHERE id=?")
            .bind(storage_path)
            .bind(id)
            .execute(&mut *tx)
            .await?
            .rows_affected()
            == 1;
        if let Some(version_id) = latest_version_id {
            sqlx::query("UPDATE artifact_versions SET storage_path=? WHERE id=?")
                .bind(storage_path)
                .bind(version_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(updated)
    }

    pub async fn get_artifact_version(&self, version_id: &str) -> Result<Option<ArtifactVersion>> {
        let row = sqlx::query(
            "SELECT id,artifact_id,version_number,content_type,storage_path,size_bytes,checksum,\
                    parent_version_id,producing_run_id,env_snapshot_hash,materialization,\
                    capture_timing,created_at \
             FROM artifact_versions WHERE id=?",
        )
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(artifact_version_from_row).transpose()
    }

    pub async fn set_artifact_version_provenance(
        &self,
        version_id: &str,
        producing_run_id: Option<&str>,
        env_snapshot_hash: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE artifact_versions SET producing_run_id=?, env_snapshot_hash=? WHERE id=?",
        )
        .bind(producing_run_id)
        .bind(env_snapshot_hash)
        .bind(version_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Artifacts for one conversation frame, newest first.
    pub async fn list_artifacts(
        &self,
        root_frame_id: &str,
    ) -> Result<Vec<(String, String, String, String, i64)>> {
        let rows = sqlx::query(
            "SELECT id, filename, content_type, storage_path, created_at FROM artifacts \
             WHERE root_frame_id=? ORDER BY created_at DESC",
        )
        .bind(root_frame_id)
        .fetch_all(&self.pool)
        .await?;
        let mut out = vec![];
        for row in rows {
            out.push((
                row.try_get("id")?,
                row.try_get("filename")?,
                row.try_get("content_type")?,
                row.try_get("storage_path")?,
                row.try_get("created_at")?,
            ));
        }
        Ok(out)
    }

    /// Recent artifacts for one project, optionally filtered by filename.
    pub async fn search_project_artifacts(
        &self,
        project_id: &str,
        query: &str,
        limit: i64,
    ) -> Result<Vec<(String, String, String, String, i64)>> {
        let q = query.trim().to_lowercase();
        let rows = sqlx::query(
            "SELECT id, filename, content_type, storage_path, created_at FROM artifacts \
             WHERE project_id=? AND (?='' OR lower(filename) LIKE ?) \
             ORDER BY created_at DESC, filename ASC LIMIT ?",
        )
        .bind(project_id)
        .bind(&q)
        .bind(format!("%{q}%"))
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?;
        let mut out = vec![];
        for row in rows {
            out.push((
                row.try_get("id")?,
                row.try_get("filename")?,
                row.try_get("content_type")?,
                row.try_get("storage_path")?,
                row.try_get("created_at")?,
            ));
        }
        Ok(out)
    }

    /// Recent artifacts across projects for composer and command-palette search.
    /// The rows intentionally carry ownership metadata: callers must validate a
    /// cross-project path against its owner's workspace, not the current one.
    pub async fn search_artifacts(
        &self,
        project_id: Option<&str>,
        query: &str,
        limit: i64,
        artifact_id: Option<&str>,
    ) -> Result<Vec<ArtifactSearchResult>> {
        let q = query.trim().to_lowercase();
        let rows = sqlx::query(
            "SELECT a.id AS id, a.filename AS filename, a.content_type AS content_type, \
                    a.storage_path AS storage_path, a.created_at AS created_at, \
                    a.project_id AS project_id, COALESCE(p.name,'') AS project_name, \
                    COALESCE(p.workspace_dir,'') AS project_root, a.root_frame_id AS frame_id, \
                    COALESCE(f.title,'') AS frame_title, \
                    (SELECT content FROM messages m WHERE m.frame_id=a.root_frame_id AND m.role='user' ORDER BY m.seq ASC LIMIT 1) AS first_user, \
                    (SELECT size_bytes FROM artifact_versions v WHERE v.id=a.latest_version_id) AS size_bytes, \
                    CASE \
                      WHEN EXISTS (SELECT 1 FROM run_artifacts ra WHERE ra.artifact_id=a.id) THEN 'output' \
                      WHEN a.logical_key LIKE 'path:uploads/%' \
                        OR replace(a.storage_path, '\\\\', '/') LIKE replace(p.workspace_dir, '\\\\', '/') || '/uploads/%' THEN 'upload' \
                      ELSE 'artifact' \
                    END AS origin \
             FROM artifacts a JOIN projects p ON p.id=a.project_id \
             LEFT JOIN frames f ON f.id=a.root_frame_id \
             WHERE (? IS NULL OR a.project_id=?) AND (? IS NULL OR a.id=?) \
               AND (?='' OR lower(a.filename) LIKE ?) \
             ORDER BY a.created_at DESC, a.filename ASC LIMIT ?",
        )
        .bind(project_id)
        .bind(project_id)
        .bind(artifact_id)
        .bind(artifact_id)
        .bind(&q)
        .bind(format!("%{q}%"))
        .bind(limit.clamp(1, 100))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(ArtifactSearchResult {
                    id: row.try_get("id")?,
                    name: row.try_get("filename")?,
                    kind: row.try_get("content_type")?,
                    path: row.try_get("storage_path")?,
                    ts: row.try_get("created_at")?,
                    project_id: row.try_get("project_id")?,
                    project_name: row.try_get("project_name")?,
                    project_root: row.try_get("project_root")?,
                    session_id: row.try_get("frame_id")?,
                    session_title: session_display_title(
                        row.try_get::<Option<String>, _>("frame_title")?,
                        row.try_get::<Option<String>, _>("first_user")?,
                    ),
                    size_bytes: row.try_get("size_bytes")?,
                    origin: row.try_get("origin")?,
                })
            })
            .collect()
    }

    /// Root sessions across projects, newest activity first, optionally matched
    /// by their display title. Empty frames never appear in the picker.

    pub async fn get_artifact(&self, id: &str) -> Result<Option<(String, String, String, String)>> {
        let row = sqlx::query(
            "SELECT filename, content_type, storage_path, root_frame_id FROM artifacts WHERE id=?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| {
            (
                r.try_get("filename").unwrap_or_default(),
                r.try_get("content_type").unwrap_or_default(),
                r.try_get("storage_path").unwrap_or_default(),
                r.try_get("root_frame_id").unwrap_or_default(),
            )
        }))
    }

    /// Artifact ownership plus storage information for safe cross-project
    /// preview and explicit composer attachment resolution.
    pub async fn get_artifact_detail(&self, id: &str) -> Result<Option<ArtifactSearchResult>> {
        Ok(self
            .search_artifacts(None, "", 1, Some(id))
            .await?
            .into_iter()
            .next())
    }
}
