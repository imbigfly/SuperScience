use super::{
    artifact_node_id, canonical_json, canonical_json_sha256, run_node_id, ArtifactMaterialization,
    EvidenceBinding, EvidenceBindingDraft, EvidenceReproductionState, EvidenceReview,
    EvidenceReviewState, EvidenceSelectionState, EvidenceSourceKind, EvidenceSupersession,
    EvidenceVisibility, Publication, PublicationCapabilityLevel, PublicationEvidenceDrift,
    PublicationFreezeCommit, PublicationFreezePolicy, PublicationItem, PublicationItemKind,
    PublicationItemLink, PublicationReadinessReport, PublicationRevision, PublicationRevisionState,
    PublicationWaiver, ResearchEdge, ResearchNode, ResearchNodeKind, Store,
};
use anyhow::Result;
use sha2::{Digest, Sha256};
use sqlx::{Row, Sqlite, Transaction};
use std::collections::HashMap;

fn publication_node_id(publication_id: &str) -> String {
    format!("publication:{publication_id}")
}

fn publication_from_row(row: sqlx::sqlite::SqliteRow) -> Result<Publication> {
    Ok(Publication {
        id: row.try_get("id")?,
        project_id: row.try_get("project_id")?,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn publication_revision_from_row(row: sqlx::sqlite::SqliteRow) -> Result<PublicationRevision> {
    let state: String = row.try_get("state")?;
    let capability: String = row.try_get("capability_level")?;
    Ok(PublicationRevision {
        id: row.try_get("id")?,
        publication_id: row.try_get("publication_id")?,
        parent_revision_id: row.try_get("parent_revision_id")?,
        revision_number: row.try_get("revision_number")?,
        label: row.try_get("label")?,
        state: PublicationRevisionState::from_storage(&state)?,
        capability_level: PublicationCapabilityLevel::from_storage(&capability)?,
        manifest_json: row.try_get("manifest_json")?,
        manifest_sha256: row.try_get("manifest_sha256")?,
        frozen_at: row.try_get("frozen_at")?,
        published_at: row.try_get("published_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn publication_item_from_row(row: sqlx::sqlite::SqliteRow) -> Result<PublicationItem> {
    let kind: String = row.try_get("kind")?;
    Ok(PublicationItem {
        id: row.try_get("id")?,
        revision_id: row.try_get("revision_id")?,
        parent_item_id: row.try_get("parent_item_id")?,
        kind: PublicationItemKind::from_storage(&kind)?,
        title: row.try_get("title")?,
        content: row.try_get("content")?,
        ordinal: row.try_get("ordinal")?,
        metadata_json: row.try_get("metadata_json")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn publication_item_link_from_row(row: sqlx::sqlite::SqliteRow) -> Result<PublicationItemLink> {
    Ok(PublicationItemLink {
        id: row.try_get("id")?,
        revision_id: row.try_get("revision_id")?,
        source_item_id: row.try_get("source_item_id")?,
        target_item_id: row.try_get("target_item_id")?,
        relation: row.try_get("relation")?,
        created_at: row.try_get("created_at")?,
    })
}

fn evidence_binding_from_row(row: sqlx::sqlite::SqliteRow) -> Result<EvidenceBinding> {
    let source_kind: String = row.try_get("source_kind")?;
    let selection_state: String = row.try_get("selection_state")?;
    let review_state: String = row.try_get("review_state")?;
    let reproduction_state: String = row.try_get("reproduction_state")?;
    let visibility: String = row.try_get("visibility")?;
    Ok(EvidenceBinding {
        id: row.try_get("id")?,
        revision_id: row.try_get("revision_id")?,
        item_id: row.try_get("item_id")?,
        source_kind: EvidenceSourceKind::from_storage(&source_kind)?,
        source_id: row.try_get("source_id")?,
        artifact_version_id: row.try_get("artifact_version_id")?,
        run_id: row.try_get("run_id")?,
        external_resource_id: row.try_get("external_resource_id")?,
        purpose: row.try_get("purpose")?,
        supported_claim_item_id: row.try_get("supported_claim_item_id")?,
        selection_state: EvidenceSelectionState::from_storage(&selection_state)?,
        review_state: EvidenceReviewState::from_storage(&review_state)?,
        reproduction_state: EvidenceReproductionState::from_storage(&reproduction_state)?,
        visibility: EvidenceVisibility::from_storage(&visibility)?,
        source_snapshot_json: row.try_get("source_snapshot_json")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

const PUBLICATION_REVISION_COLUMNS: &str = "id,publication_id,parent_revision_id,revision_number,\
    label,state,capability_level,manifest_json,manifest_sha256,frozen_at,published_at,created_at,\
    updated_at";
const EVIDENCE_BINDING_COLUMNS: &str = "id,revision_id,item_id,source_kind,source_id,\
    artifact_version_id,run_id,external_resource_id,purpose,supported_claim_item_id,\
    selection_state,review_state,reproduction_state,visibility,source_snapshot_json,created_at,\
    updated_at";

async fn draft_revision_project(
    tx: &mut Transaction<'_, Sqlite>,
    revision_id: &str,
) -> Result<String> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT publication.project_id,revision.state \
         FROM publication_revisions revision \
         JOIN publications publication ON publication.id=revision.publication_id \
         WHERE revision.id=?",
    )
    .bind(revision_id)
    .fetch_optional(&mut **tx)
    .await?;
    match row {
        Some((project_id, state)) if state == "draft" => Ok(project_id),
        Some(_) => anyhow::bail!("Publication revision is immutable"),
        None => anyhow::bail!("Publication revision not found"),
    }
}

struct ResolvedEvidenceSource {
    artifact_version_id: Option<String>,
    run_id: Option<String>,
    external_resource_id: Option<String>,
    snapshot_json: String,
    target_node_id: String,
    target_kind: ResearchNodeKind,
    target_title: String,
}

async fn resolve_evidence_source(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &str,
    kind: EvidenceSourceKind,
    source_id: &str,
) -> Result<ResolvedEvidenceSource> {
    match kind {
        EvidenceSourceKind::ArtifactVersion => {
            let row = sqlx::query(
                "SELECT version.id,version.artifact_id,version.version_number,\
                        version.content_type,version.size_bytes,version.checksum,\
                        version.producing_run_id,version.env_snapshot_hash,\
                        version.materialization,version.capture_timing,\
                        artifact.filename,artifact.logical_key \
                 FROM artifact_versions version \
                 JOIN artifacts artifact ON artifact.id=version.artifact_id \
                 WHERE version.id=? AND artifact.project_id=?",
            )
            .bind(source_id)
            .bind(project_id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("ArtifactVersion evidence must belong to the Publication project")
            })?;
            let artifact_id: String = row.try_get("artifact_id")?;
            let filename: String = row.try_get("filename")?;
            let snapshot = serde_json::json!({
                "source_kind": "artifact_version",
                "source_id": source_id,
                "artifact_id": artifact_id,
                "version_number": row.try_get::<i64, _>("version_number")?,
                "filename": filename,
                "content_type": row.try_get::<String, _>("content_type")?,
                "size_bytes": row.try_get::<Option<i64>, _>("size_bytes")?,
                "checksum": row.try_get::<Option<String>, _>("checksum")?,
                "producing_run_id": row.try_get::<Option<String>, _>("producing_run_id")?,
                "env_snapshot_hash": row.try_get::<Option<String>, _>("env_snapshot_hash")?,
                "materialization": row.try_get::<String, _>("materialization")?,
                "capture_timing": row.try_get::<String, _>("capture_timing")?,
                "logical_key": row.try_get::<Option<String>, _>("logical_key")?,
            });
            Ok(ResolvedEvidenceSource {
                artifact_version_id: Some(source_id.to_string()),
                run_id: None,
                external_resource_id: None,
                snapshot_json: canonical_json(&snapshot),
                target_node_id: artifact_node_id(&artifact_id),
                target_kind: ResearchNodeKind::Artifact,
                target_title: filename,
            })
        }
        EvidenceSourceKind::Run => {
            let row = sqlx::query(
                "SELECT id,title,kind,status,context_id,command,created_at \
                 FROM runs WHERE id=? AND project_id=?",
            )
            .bind(source_id)
            .bind(project_id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Run evidence must belong to the Publication project")
            })?;
            let title: String = row.try_get("title")?;
            let command: Option<String> = row.try_get("command")?;
            let command_sha256 = command.map(|command| {
                let mut digest = Sha256::new();
                digest.update(command.as_bytes());
                hex::encode(digest.finalize())
            });
            let snapshot = serde_json::json!({
                "source_kind": "run",
                "source_id": source_id,
                "title": title,
                "kind": row.try_get::<String, _>("kind")?,
                "status": row.try_get::<String, _>("status")?,
                "context_id": row.try_get::<String, _>("context_id")?,
                "command_sha256": command_sha256,
                "created_at": row.try_get::<i64, _>("created_at")?,
            });
            Ok(ResolvedEvidenceSource {
                artifact_version_id: None,
                run_id: Some(source_id.to_string()),
                external_resource_id: None,
                snapshot_json: canonical_json(&snapshot),
                target_node_id: run_node_id(source_id),
                target_kind: ResearchNodeKind::Run,
                target_title: title,
            })
        }
        _ => anyhow::bail!("Publication v0.30 accepts only exact ArtifactVersion and Run evidence"),
    }
}

async fn delete_revision_children(
    tx: &mut Transaction<'_, Sqlite>,
    revision_id: &str,
) -> Result<()> {
    sqlx::query(
        "DELETE FROM research_edges WHERE id IN (\
           SELECT 'publication-evidence:' || id FROM evidence_bindings WHERE revision_id=?\
         )",
    )
    .bind(revision_id)
    .execute(&mut **tx)
    .await?;
    for statement in [
        "DELETE FROM publication_freeze_attempts WHERE revision_id=?",
        "DELETE FROM capsule_builds WHERE revision_id=?",
        "DELETE FROM publication_readiness_reports WHERE revision_id=?",
        "DELETE FROM publication_waivers WHERE revision_id=?",
        "DELETE FROM evidence_reviews WHERE binding_id IN \
           (SELECT id FROM evidence_bindings WHERE revision_id=?)",
        "DELETE FROM evidence_supersessions WHERE revision_id=?",
        "DELETE FROM evidence_bindings WHERE revision_id=?",
        "DELETE FROM publication_item_links WHERE revision_id=?",
        "DELETE FROM publication_items WHERE revision_id=?",
    ] {
        sqlx::query(statement)
            .bind(revision_id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

impl Store {
    pub async fn create_publication(
        &self,
        id: &str,
        project_id: &str,
        title: &str,
        description: &str,
    ) -> Result<Publication> {
        if id.trim().is_empty() || project_id.trim().is_empty() || title.trim().is_empty() {
            anyhow::bail!("Publication requires identity, project, and title");
        }
        let now = chrono::Utc::now().timestamp();
        let inserted = sqlx::query(
            "INSERT INTO publications(id,project_id,title,description,created_at,updated_at) \
             SELECT ?,id,?,?,?,? FROM projects WHERE id=?",
        )
        .bind(id)
        .bind(title.trim())
        .bind(description)
        .bind(now)
        .bind(now)
        .bind(project_id)
        .execute(&self.pool)
        .await?;
        if inserted.rows_affected() != 1 {
            anyhow::bail!("Publication project not found");
        }

        let mut node = ResearchNode::new(
            publication_node_id(id),
            project_id,
            ResearchNodeKind::Paper,
            title.trim(),
        )?;
        node.ref_id = Some(id.to_string());
        node.metadata_json = r#"{"projection":"publication"}"#.into();
        self.save_research_node(&node).await?;
        self.get_publication(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Publication was not persisted"))
    }

    pub async fn get_publication(&self, id: &str) -> Result<Option<Publication>> {
        let row = sqlx::query(
            "SELECT id,project_id,title,description,created_at,updated_at \
             FROM publications WHERE id=?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(publication_from_row).transpose()
    }

    pub async fn list_publications(&self, project_id: &str) -> Result<Vec<Publication>> {
        let rows = sqlx::query(
            "SELECT id,project_id,title,description,created_at,updated_at \
             FROM publications WHERE project_id=? ORDER BY updated_at DESC,id",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(publication_from_row).collect()
    }

    pub async fn update_publication(&self, id: &str, title: &str, description: &str) -> Result<()> {
        if title.trim().is_empty() {
            anyhow::bail!("Publication title cannot be empty");
        }
        let updated =
            sqlx::query("UPDATE publications SET title=?,description=?,updated_at=? WHERE id=?")
                .bind(title.trim())
                .bind(description)
                .bind(chrono::Utc::now().timestamp())
                .bind(id)
                .execute(&self.pool)
                .await?;
        if updated.rows_affected() != 1 {
            anyhow::bail!("Publication not found");
        }
        if let Some(publication) = self.get_publication(id).await? {
            let mut node = ResearchNode::new(
                publication_node_id(id),
                &publication.project_id,
                ResearchNodeKind::Paper,
                &publication.title,
            )?;
            node.ref_id = Some(id.to_string());
            node.metadata_json = r#"{"projection":"publication"}"#.into();
            self.save_research_node(&node).await?;
        }
        Ok(())
    }

    pub async fn delete_publication(&self, id: &str) -> Result<()> {
        let mut tx = self.begin_write().await?;
        let publication: Option<String> =
            sqlx::query_scalar("SELECT project_id FROM publications WHERE id=?")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        if publication.is_none() {
            anyhow::bail!("Publication not found");
        }
        let revisions = sqlx::query(
            "SELECT id,state FROM publication_revisions \
             WHERE publication_id=? ORDER BY revision_number DESC",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        for row in &revisions {
            let state: String = row.try_get("state")?;
            if state != "draft" {
                anyhow::bail!("Publication with immutable revisions cannot be deleted");
            }
        }
        for row in revisions {
            let revision_id: String = row.try_get("id")?;
            delete_revision_children(&mut tx, &revision_id).await?;
            sqlx::query("DELETE FROM publication_revisions WHERE id=?")
                .bind(revision_id)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("DELETE FROM research_edges WHERE source_id=? OR target_id=?")
            .bind(publication_node_id(id))
            .bind(publication_node_id(id))
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM research_nodes WHERE id=?")
            .bind(publication_node_id(id))
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM publications WHERE id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn create_publication_revision(
        &self,
        id: &str,
        publication_id: &str,
        parent_revision_id: Option<&str>,
        label: &str,
    ) -> Result<PublicationRevision> {
        if id.trim().is_empty() || publication_id.trim().is_empty() || label.trim().is_empty() {
            anyhow::bail!("Publication revision requires identity, Publication, and label");
        }
        let now = chrono::Utc::now().timestamp();
        let mut tx = self.begin_write().await?;
        let publication_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM publications WHERE id=?)")
                .bind(publication_id)
                .fetch_one(&mut *tx)
                .await?;
        if !publication_exists {
            anyhow::bail!("Publication not found");
        }
        if let Some(parent_id) = parent_revision_id {
            let parent_publication: Option<String> =
                sqlx::query_scalar("SELECT publication_id FROM publication_revisions WHERE id=?")
                    .bind(parent_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            if parent_publication.as_deref() != Some(publication_id) {
                anyhow::bail!("Parent revision must belong to the same Publication");
            }
        }
        let revision_number: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(revision_number),0)+1 FROM publication_revisions \
             WHERE publication_id=?",
        )
        .bind(publication_id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO publication_revisions(\
               id,publication_id,parent_revision_id,revision_number,label,state,capability_level,\
               manifest_json,manifest_sha256,frozen_at,published_at,created_at,updated_at\
             ) VALUES(?,?,?,?,?,'draft','archived',NULL,NULL,NULL,NULL,?,?)",
        )
        .bind(id)
        .bind(publication_id)
        .bind(parent_revision_id)
        .bind(revision_number)
        .bind(label.trim())
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.get_publication_revision(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Publication revision was not persisted"))
    }

    pub async fn get_publication_revision(&self, id: &str) -> Result<Option<PublicationRevision>> {
        let row = sqlx::query(&format!(
            "SELECT {PUBLICATION_REVISION_COLUMNS} FROM publication_revisions WHERE id=?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(publication_revision_from_row).transpose()
    }

    pub async fn list_publication_revisions(
        &self,
        publication_id: &str,
    ) -> Result<Vec<PublicationRevision>> {
        let rows = sqlx::query(&format!(
            "SELECT {PUBLICATION_REVISION_COLUMNS} FROM publication_revisions \
             WHERE publication_id=? ORDER BY revision_number DESC"
        ))
        .bind(publication_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(publication_revision_from_row)
            .collect()
    }

    pub async fn update_draft_publication_revision(&self, id: &str, label: &str) -> Result<()> {
        if label.trim().is_empty() {
            anyhow::bail!("Publication revision label cannot be empty");
        }
        let updated = sqlx::query(
            "UPDATE publication_revisions SET label=?,updated_at=? \
             WHERE id=? AND state='draft'",
        )
        .bind(label.trim())
        .bind(chrono::Utc::now().timestamp())
        .bind(id)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() != 1 {
            anyhow::bail!("Draft Publication revision not found");
        }
        Ok(())
    }

    pub async fn delete_draft_publication_revision(&self, id: &str) -> Result<()> {
        let mut tx = self.begin_write().await?;
        let state: Option<String> =
            sqlx::query_scalar("SELECT state FROM publication_revisions WHERE id=?")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        match state.as_deref() {
            Some("draft") => {}
            Some(_) => anyhow::bail!("Publication revision is immutable"),
            None => anyhow::bail!("Publication revision not found"),
        }
        let has_children: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM publication_revisions WHERE parent_revision_id=?)",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if has_children {
            anyhow::bail!("Publication revision with descendants cannot be deleted");
        }
        delete_revision_children(&mut tx, id).await?;
        sqlx::query("DELETE FROM publication_revisions WHERE id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn save_publication_item(&self, item: &PublicationItem) -> Result<()> {
        if item.id.trim().is_empty()
            || item.revision_id.trim().is_empty()
            || item.title.trim().is_empty()
            || item.ordinal < 0
        {
            anyhow::bail!("Publication item requires identity, revision, title, and ordinal");
        }
        serde_json::from_str::<serde_json::Value>(&item.metadata_json)
            .map_err(|_| anyhow::anyhow!("Publication item metadata must be valid JSON"))?;
        if item.parent_item_id.as_deref() == Some(item.id.as_str()) {
            anyhow::bail!("Publication item cannot parent itself");
        }

        let mut tx = self.begin_write().await?;
        draft_revision_project(&mut tx, &item.revision_id).await?;
        let existing_revision: Option<String> =
            sqlx::query_scalar("SELECT revision_id FROM publication_items WHERE id=?")
                .bind(&item.id)
                .fetch_optional(&mut *tx)
                .await?;
        if existing_revision
            .as_deref()
            .is_some_and(|revision_id| revision_id != item.revision_id)
        {
            anyhow::bail!("Publication item cannot move between revisions");
        }
        if let Some(parent_id) = item.parent_item_id.as_deref() {
            let parent_revision: Option<String> =
                sqlx::query_scalar("SELECT revision_id FROM publication_items WHERE id=?")
                    .bind(parent_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            if parent_revision.as_deref() != Some(item.revision_id.as_str()) {
                anyhow::bail!("Publication item parent must belong to the revision");
            }
            let cycle: bool = sqlx::query_scalar(
                "WITH RECURSIVE ancestors(id) AS (\
                   SELECT ? \
                   UNION \
                   SELECT item.parent_item_id FROM publication_items item \
                   JOIN ancestors parent ON item.id=parent.id \
                   WHERE item.parent_item_id IS NOT NULL\
                 ) SELECT EXISTS(SELECT 1 FROM ancestors WHERE id=?)",
            )
            .bind(parent_id)
            .bind(&item.id)
            .fetch_one(&mut *tx)
            .await?;
            if cycle {
                anyhow::bail!("Publication item hierarchy cannot contain a cycle");
            }
        }
        let now = chrono::Utc::now().timestamp();
        let created_at = if item.created_at == 0 {
            now
        } else {
            item.created_at
        };
        sqlx::query(
            "INSERT INTO publication_items(\
               id,revision_id,parent_item_id,kind,title,content,ordinal,metadata_json,\
               created_at,updated_at\
             ) VALUES(?,?,?,?,?,?,?,?,?,?) \
             ON CONFLICT(id) DO UPDATE SET \
               parent_item_id=excluded.parent_item_id,kind=excluded.kind,title=excluded.title,\
               content=excluded.content,ordinal=excluded.ordinal,\
               metadata_json=excluded.metadata_json,updated_at=excluded.updated_at",
        )
        .bind(&item.id)
        .bind(&item.revision_id)
        .bind(item.parent_item_id.as_deref())
        .bind(item.kind.as_str())
        .bind(item.title.trim())
        .bind(&item.content)
        .bind(item.ordinal)
        .bind(canonical_json(
            &serde_json::from_str(&item.metadata_json).expect("validated JSON"),
        ))
        .bind(created_at)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_publication_items(&self, revision_id: &str) -> Result<Vec<PublicationItem>> {
        let rows = sqlx::query(
            "SELECT id,revision_id,parent_item_id,kind,title,content,ordinal,metadata_json,\
                    created_at,updated_at \
             FROM publication_items WHERE revision_id=? \
             ORDER BY COALESCE(parent_item_id,''),ordinal,id",
        )
        .bind(revision_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(publication_item_from_row).collect()
    }

    pub async fn delete_publication_item(&self, id: &str) -> Result<()> {
        let mut tx = self.begin_write().await?;
        let revision_id: Option<String> =
            sqlx::query_scalar("SELECT revision_id FROM publication_items WHERE id=?")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        let revision_id =
            revision_id.ok_or_else(|| anyhow::anyhow!("Publication item not found"))?;
        draft_revision_project(&mut tx, &revision_id).await?;
        sqlx::query(
            "WITH RECURSIVE descendants(id) AS (\
               SELECT id FROM publication_items WHERE id=? \
               UNION ALL \
               SELECT child.id FROM publication_items child \
               JOIN descendants parent ON child.parent_item_id=parent.id\
             ) \
             DELETE FROM research_edges WHERE id IN (\
               SELECT 'publication-evidence:' || binding.id \
               FROM evidence_bindings binding \
               WHERE binding.item_id IN (SELECT id FROM descendants)\
             )",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM publication_items WHERE id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn save_publication_item_link(&self, link: &PublicationItemLink) -> Result<()> {
        if link.id.trim().is_empty()
            || link.revision_id.trim().is_empty()
            || link.source_item_id.trim().is_empty()
            || link.target_item_id.trim().is_empty()
            || link.relation.trim().is_empty()
            || link.source_item_id == link.target_item_id
        {
            anyhow::bail!("Publication item link requires distinct items and a relation");
        }
        let mut tx = self.begin_write().await?;
        draft_revision_project(&mut tx, &link.revision_id).await?;
        let endpoint_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM publication_items \
             WHERE revision_id=? AND id IN (?,?)",
        )
        .bind(&link.revision_id)
        .bind(&link.source_item_id)
        .bind(&link.target_item_id)
        .fetch_one(&mut *tx)
        .await?;
        if endpoint_count != 2 {
            anyhow::bail!("Publication item link must stay inside one revision");
        }
        let existing_revision: Option<String> =
            sqlx::query_scalar("SELECT revision_id FROM publication_item_links WHERE id=?")
                .bind(&link.id)
                .fetch_optional(&mut *tx)
                .await?;
        if existing_revision
            .as_deref()
            .is_some_and(|revision_id| revision_id != link.revision_id)
        {
            anyhow::bail!("Publication item link cannot move between revisions");
        }
        sqlx::query(
            "INSERT INTO publication_item_links(\
               id,revision_id,source_item_id,target_item_id,relation,created_at\
             ) VALUES(?,?,?,?,?,?) \
             ON CONFLICT(id) DO UPDATE SET \
               source_item_id=excluded.source_item_id,target_item_id=excluded.target_item_id,\
               relation=excluded.relation",
        )
        .bind(&link.id)
        .bind(&link.revision_id)
        .bind(&link.source_item_id)
        .bind(&link.target_item_id)
        .bind(link.relation.trim())
        .bind(if link.created_at == 0 {
            chrono::Utc::now().timestamp()
        } else {
            link.created_at
        })
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_publication_item_links(
        &self,
        revision_id: &str,
    ) -> Result<Vec<PublicationItemLink>> {
        let rows = sqlx::query(
            "SELECT id,revision_id,source_item_id,target_item_id,relation,created_at \
             FROM publication_item_links WHERE revision_id=? ORDER BY created_at,id",
        )
        .bind(revision_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(publication_item_link_from_row)
            .collect()
    }

    pub async fn delete_publication_item_link(&self, id: &str) -> Result<()> {
        let mut tx = self.begin_write().await?;
        let revision_id: Option<String> =
            sqlx::query_scalar("SELECT revision_id FROM publication_item_links WHERE id=?")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        let revision_id =
            revision_id.ok_or_else(|| anyhow::anyhow!("Publication item link not found"))?;
        draft_revision_project(&mut tx, &revision_id).await?;
        sqlx::query("DELETE FROM publication_item_links WHERE id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn save_evidence_binding(
        &self,
        draft: &EvidenceBindingDraft,
    ) -> Result<EvidenceBinding> {
        if draft.id.trim().is_empty()
            || draft.revision_id.trim().is_empty()
            || draft.source_id.trim().is_empty()
        {
            anyhow::bail!("Evidence binding requires identity, revision, and exact source");
        }
        let now = chrono::Utc::now().timestamp();
        let mut tx = self.begin_write().await?;
        let project_id = draft_revision_project(&mut tx, &draft.revision_id).await?;
        let source =
            resolve_evidence_source(&mut tx, &project_id, draft.source_kind, &draft.source_id)
                .await?;
        let existing = sqlx::query(
            "SELECT revision_id,source_kind,source_id,review_state,reproduction_state,\
                    source_snapshot_json,created_at \
             FROM evidence_bindings WHERE id=?",
        )
        .bind(&draft.id)
        .fetch_optional(&mut *tx)
        .await?;
        let (review_state, reproduction_state, source_snapshot_json, created_at) =
            if let Some(row) = existing {
                let revision_id: String = row.try_get("revision_id")?;
                let source_kind: String = row.try_get("source_kind")?;
                let source_id: String = row.try_get("source_id")?;
                if revision_id != draft.revision_id
                    || source_kind != draft.source_kind.as_str()
                    || source_id != draft.source_id
                {
                    anyhow::bail!("Evidence binding exact source and revision cannot be changed");
                }
                (
                    row.try_get::<String, _>("review_state")?,
                    row.try_get::<String, _>("reproduction_state")?,
                    row.try_get::<String, _>("source_snapshot_json")?,
                    row.try_get::<i64, _>("created_at")?,
                )
            } else {
                (
                    EvidenceReviewState::Unreviewed.as_str().to_string(),
                    EvidenceReproductionState::NotRun.as_str().to_string(),
                    source.snapshot_json,
                    now,
                )
            };
        sqlx::query(
            "INSERT INTO evidence_bindings(\
               id,revision_id,item_id,source_kind,source_id,artifact_version_id,run_id,\
               external_resource_id,purpose,supported_claim_item_id,selection_state,review_state,\
               reproduction_state,visibility,source_snapshot_json,created_at,updated_at\
             ) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) \
             ON CONFLICT(id) DO UPDATE SET \
               item_id=excluded.item_id,purpose=excluded.purpose,\
               supported_claim_item_id=excluded.supported_claim_item_id,\
               selection_state=excluded.selection_state,visibility=excluded.visibility,\
               updated_at=excluded.updated_at",
        )
        .bind(&draft.id)
        .bind(&draft.revision_id)
        .bind(draft.item_id.as_deref())
        .bind(draft.source_kind.as_str())
        .bind(&draft.source_id)
        .bind(source.artifact_version_id.as_deref())
        .bind(source.run_id.as_deref())
        .bind(source.external_resource_id.as_deref())
        .bind(draft.purpose.trim())
        .bind(draft.supported_claim_item_id.as_deref())
        .bind(draft.selection_state.as_str())
        .bind(review_state)
        .bind(reproduction_state)
        .bind(draft.visibility.as_str())
        .bind(source_snapshot_json)
        .bind(created_at)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        self.sync_evidence_projection(
            &draft.id,
            &project_id,
            &draft.revision_id,
            draft.item_id.as_deref(),
            draft.source_kind,
            &draft.source_id,
            &source.target_node_id,
            source.target_kind,
            &source.target_title,
        )
        .await?;
        self.get_evidence_binding(&draft.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Evidence binding was not persisted"))
    }

    #[allow(clippy::too_many_arguments)]
    async fn sync_evidence_projection(
        &self,
        binding_id: &str,
        project_id: &str,
        revision_id: &str,
        item_id: Option<&str>,
        source_kind: EvidenceSourceKind,
        source_id: &str,
        target_node_id: &str,
        target_kind: ResearchNodeKind,
        target_title: &str,
    ) -> Result<()> {
        let publication: (String, String) = sqlx::query_as(
            "SELECT publication.id,publication.title \
             FROM publication_revisions revision \
             JOIN publications publication ON publication.id=revision.publication_id \
             WHERE revision.id=?",
        )
        .bind(revision_id)
        .fetch_one(&self.pool)
        .await?;
        let mut publication_node = ResearchNode::new(
            publication_node_id(&publication.0),
            project_id,
            ResearchNodeKind::Paper,
            &publication.1,
        )?;
        publication_node.ref_id = Some(publication.0);
        publication_node.metadata_json = r#"{"projection":"publication"}"#.into();
        self.save_research_node(&publication_node).await?;

        let mut target = ResearchNode::new(target_node_id, project_id, target_kind, target_title)?;
        target.ref_id = Some(match source_kind {
            EvidenceSourceKind::ArtifactVersion => {
                sqlx::query_scalar("SELECT artifact_id FROM artifact_versions WHERE id=?")
                    .bind(source_id)
                    .fetch_one(&self.pool)
                    .await?
            }
            EvidenceSourceKind::Run => source_id.to_string(),
            _ => source_id.to_string(),
        });
        self.save_research_node(&target).await?;

        let mut edge = ResearchEdge::new(
            format!("publication-evidence:{binding_id}"),
            project_id,
            &publication_node.id,
            target_node_id,
            "uses_evidence",
        )?;
        edge.metadata_json = canonical_json(&serde_json::json!({
            "binding_id": binding_id,
            "revision_id": revision_id,
            "item_id": item_id,
            "source_kind": source_kind.as_str(),
            "source_id": source_id,
        }));
        self.save_research_edge(&edge).await
    }

    pub async fn get_evidence_binding(&self, id: &str) -> Result<Option<EvidenceBinding>> {
        let row = sqlx::query(&format!(
            "SELECT {EVIDENCE_BINDING_COLUMNS} FROM evidence_bindings WHERE id=?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(evidence_binding_from_row).transpose()
    }

    pub async fn list_evidence_bindings(&self, revision_id: &str) -> Result<Vec<EvidenceBinding>> {
        let rows = sqlx::query(&format!(
            "SELECT {EVIDENCE_BINDING_COLUMNS} FROM evidence_bindings \
             WHERE revision_id=? ORDER BY created_at,id"
        ))
        .bind(revision_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(evidence_binding_from_row).collect()
    }

    pub async fn update_evidence_binding_selection(
        &self,
        id: &str,
        selection_state: EvidenceSelectionState,
        visibility: EvidenceVisibility,
    ) -> Result<()> {
        let updated = sqlx::query(
            "UPDATE evidence_bindings SET \
               selection_state=?,visibility=?,updated_at=? \
             WHERE id=? AND EXISTS(\
               SELECT 1 FROM publication_revisions revision \
               WHERE revision.id=evidence_bindings.revision_id AND revision.state='draft')",
        )
        .bind(selection_state.as_str())
        .bind(visibility.as_str())
        .bind(chrono::Utc::now().timestamp())
        .bind(id)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() != 1 {
            anyhow::bail!("Draft evidence binding not found");
        }
        Ok(())
    }

    pub async fn delete_evidence_binding(&self, id: &str) -> Result<()> {
        let mut tx = self.begin_write().await?;
        let revision_id: Option<String> =
            sqlx::query_scalar("SELECT revision_id FROM evidence_bindings WHERE id=?")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        let revision_id =
            revision_id.ok_or_else(|| anyhow::anyhow!("Evidence binding not found"))?;
        draft_revision_project(&mut tx, &revision_id).await?;
        sqlx::query("DELETE FROM research_edges WHERE id=?")
            .bind(format!("publication-evidence:{id}"))
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM evidence_bindings WHERE id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn save_evidence_review(&self, review: &EvidenceReview) -> Result<()> {
        if review.id.trim().is_empty()
            || review.binding_id.trim().is_empty()
            || review.reviewer.trim().is_empty()
            || review.method.trim().is_empty()
            || review.result.trim().is_empty()
        {
            anyhow::bail!("Evidence review requires identity, reviewer, method, and result");
        }
        for (label, value) in [
            ("environment", &review.environment_json),
            ("comparator", &review.comparator_json),
            ("tolerance", &review.tolerance_json),
            ("report", &review.report_json),
        ] {
            if serde_json::from_str::<serde_json::Value>(value).is_err() {
                anyhow::bail!("Evidence review {label} must be valid JSON");
            }
        }
        let mut tx = self.begin_write().await?;
        let revision_id: Option<String> =
            sqlx::query_scalar("SELECT revision_id FROM evidence_bindings WHERE id=?")
                .bind(&review.binding_id)
                .fetch_optional(&mut *tx)
                .await?;
        let revision_id =
            revision_id.ok_or_else(|| anyhow::anyhow!("Evidence binding not found"))?;
        draft_revision_project(&mut tx, &revision_id).await?;
        let existing_binding: Option<String> =
            sqlx::query_scalar("SELECT binding_id FROM evidence_reviews WHERE id=?")
                .bind(&review.id)
                .fetch_optional(&mut *tx)
                .await?;
        if existing_binding
            .as_deref()
            .is_some_and(|binding_id| binding_id != review.binding_id)
        {
            anyhow::bail!("Evidence review cannot move between bindings");
        }
        sqlx::query(
            "INSERT INTO evidence_reviews(\
               id,binding_id,reviewer,method,verified_at,environment_json,comparator_json,\
               tolerance_json,result,report_json,created_at\
             ) VALUES(?,?,?,?,?,?,?,?,?,?,?) \
             ON CONFLICT(id) DO UPDATE SET \
               reviewer=excluded.reviewer,method=excluded.method,verified_at=excluded.verified_at,\
               environment_json=excluded.environment_json,\
               comparator_json=excluded.comparator_json,\
               tolerance_json=excluded.tolerance_json,result=excluded.result,\
               report_json=excluded.report_json",
        )
        .bind(&review.id)
        .bind(&review.binding_id)
        .bind(review.reviewer.trim())
        .bind(review.method.trim())
        .bind(review.verified_at)
        .bind(canonical_json(
            &serde_json::from_str(&review.environment_json).expect("validated JSON"),
        ))
        .bind(canonical_json(
            &serde_json::from_str(&review.comparator_json).expect("validated JSON"),
        ))
        .bind(canonical_json(
            &serde_json::from_str(&review.tolerance_json).expect("validated JSON"),
        ))
        .bind(review.result.trim())
        .bind(canonical_json(
            &serde_json::from_str(&review.report_json).expect("validated JSON"),
        ))
        .bind(if review.created_at == 0 {
            chrono::Utc::now().timestamp()
        } else {
            review.created_at
        })
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE evidence_bindings SET review_state='reviewed',updated_at=? WHERE id=?")
            .bind(chrono::Utc::now().timestamp())
            .bind(&review.binding_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_evidence_reviews(&self, binding_id: &str) -> Result<Vec<EvidenceReview>> {
        let rows = sqlx::query(
            "SELECT id,binding_id,reviewer,method,verified_at,environment_json,comparator_json,\
                    tolerance_json,result,report_json,created_at \
             FROM evidence_reviews WHERE binding_id=? ORDER BY verified_at,id",
        )
        .bind(binding_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(EvidenceReview {
                    id: row.try_get("id")?,
                    binding_id: row.try_get("binding_id")?,
                    reviewer: row.try_get("reviewer")?,
                    method: row.try_get("method")?,
                    verified_at: row.try_get("verified_at")?,
                    environment_json: row.try_get("environment_json")?,
                    comparator_json: row.try_get("comparator_json")?,
                    tolerance_json: row.try_get("tolerance_json")?,
                    result: row.try_get("result")?,
                    report_json: row.try_get("report_json")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect()
    }

    pub async fn save_evidence_supersession(
        &self,
        supersession: &EvidenceSupersession,
    ) -> Result<()> {
        if supersession.id.trim().is_empty()
            || supersession.revision_id.trim().is_empty()
            || supersession.old_binding_id.trim().is_empty()
            || supersession.new_binding_id.trim().is_empty()
            || supersession.old_binding_id == supersession.new_binding_id
        {
            anyhow::bail!("Evidence supersession requires two distinct bindings");
        }
        let mut tx = self.begin_write().await?;
        draft_revision_project(&mut tx, &supersession.revision_id).await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM evidence_bindings \
             WHERE revision_id=? AND id IN (?,?)",
        )
        .bind(&supersession.revision_id)
        .bind(&supersession.old_binding_id)
        .bind(&supersession.new_binding_id)
        .fetch_one(&mut *tx)
        .await?;
        if count != 2 {
            anyhow::bail!("Evidence supersession must stay inside one revision");
        }
        sqlx::query(
            "INSERT INTO evidence_supersessions(\
               id,revision_id,old_binding_id,new_binding_id,reason,created_at\
             ) VALUES(?,?,?,?,?,?) \
             ON CONFLICT(revision_id,old_binding_id) DO UPDATE SET \
               new_binding_id=excluded.new_binding_id,reason=excluded.reason",
        )
        .bind(&supersession.id)
        .bind(&supersession.revision_id)
        .bind(&supersession.old_binding_id)
        .bind(&supersession.new_binding_id)
        .bind(supersession.reason.trim())
        .bind(if supersession.created_at == 0 {
            chrono::Utc::now().timestamp()
        } else {
            supersession.created_at
        })
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_evidence_supersessions(
        &self,
        revision_id: &str,
    ) -> Result<Vec<EvidenceSupersession>> {
        let rows = sqlx::query(
            "SELECT id,revision_id,old_binding_id,new_binding_id,reason,created_at \
             FROM evidence_supersessions WHERE revision_id=? ORDER BY created_at,id",
        )
        .bind(revision_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(EvidenceSupersession {
                    id: row.try_get("id")?,
                    revision_id: row.try_get("revision_id")?,
                    old_binding_id: row.try_get("old_binding_id")?,
                    new_binding_id: row.try_get("new_binding_id")?,
                    reason: row.try_get("reason")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect()
    }

    pub async fn save_publication_waiver(&self, waiver: &PublicationWaiver) -> Result<()> {
        if waiver.id.trim().is_empty()
            || waiver.revision_id.trim().is_empty()
            || waiver.finding_code.trim().is_empty()
            || waiver.author.trim().is_empty()
            || waiver.reason.trim().is_empty()
        {
            anyhow::bail!("Publication waiver requires finding, author, and reason");
        }
        let mut tx = self.begin_write().await?;
        draft_revision_project(&mut tx, &waiver.revision_id).await?;
        sqlx::query(
            "INSERT INTO publication_waivers(\
               id,revision_id,finding_code,author,reason,created_at\
             ) VALUES(?,?,?,?,?,?) \
             ON CONFLICT(revision_id,finding_code) DO UPDATE SET \
               author=excluded.author,reason=excluded.reason,created_at=excluded.created_at",
        )
        .bind(&waiver.id)
        .bind(&waiver.revision_id)
        .bind(waiver.finding_code.trim())
        .bind(waiver.author.trim())
        .bind(waiver.reason.trim())
        .bind(if waiver.created_at == 0 {
            chrono::Utc::now().timestamp()
        } else {
            waiver.created_at
        })
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_publication_waivers(
        &self,
        revision_id: &str,
    ) -> Result<Vec<PublicationWaiver>> {
        let rows = sqlx::query(
            "SELECT id,revision_id,finding_code,author,reason,created_at \
             FROM publication_waivers WHERE revision_id=? ORDER BY finding_code,id",
        )
        .bind(revision_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(PublicationWaiver {
                    id: row.try_get("id")?,
                    revision_id: row.try_get("revision_id")?,
                    finding_code: row.try_get("finding_code")?,
                    author: row.try_get("author")?,
                    reason: row.try_get("reason")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect()
    }

    pub async fn begin_publication_freeze(
        &self,
        revision_id: &str,
        attempt_id: &str,
        policy: &PublicationFreezePolicy,
    ) -> Result<()> {
        if revision_id.trim().is_empty() || attempt_id.trim().is_empty() {
            anyhow::bail!("Publication freeze requires revision and attempt identities");
        }
        let policy_json = canonical_json(&serde_json::to_value(policy)?);
        let now = chrono::Utc::now().timestamp();
        let mut tx = self.begin_write().await?;
        let updated = sqlx::query(
            "UPDATE publication_revisions SET state='freezing',updated_at=? \
             WHERE id=? AND state='draft'",
        )
        .bind(now)
        .bind(revision_id)
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() != 1 {
            anyhow::bail!("Draft Publication revision not found or already being frozen");
        }
        sqlx::query(
            "INSERT INTO publication_freeze_attempts(\
               id,revision_id,target_visibility,policy_json,started_at\
             ) VALUES(?,?,?,?,?)",
        )
        .bind(attempt_id)
        .bind(revision_id)
        .bind(policy.target_visibility.as_str())
        .bind(policy_json)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn abort_publication_freeze(
        &self,
        revision_id: &str,
        attempt_id: &str,
    ) -> Result<bool> {
        let now = chrono::Utc::now().timestamp();
        let mut tx = self.begin_write().await?;
        let removed =
            sqlx::query("DELETE FROM publication_freeze_attempts WHERE id=? AND revision_id=?")
                .bind(attempt_id)
                .bind(revision_id)
                .execute(&mut *tx)
                .await?;
        if removed.rows_affected() != 1 {
            tx.rollback().await?;
            return Ok(false);
        }
        let updated = sqlx::query(
            "UPDATE publication_revisions SET state='draft',updated_at=? \
             WHERE id=? AND state='freezing'",
        )
        .bind(now)
        .bind(revision_id)
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() != 1 {
            anyhow::bail!("Publication freeze state changed before abort");
        }
        tx.commit().await?;
        Ok(true)
    }

    pub async fn recover_stale_publication_freezes(
        &self,
        started_before: i64,
    ) -> Result<Vec<String>> {
        let mut tx = self.begin_write().await?;
        let revisions: Vec<String> = sqlx::query_scalar(
            "SELECT revision_id FROM publication_freeze_attempts \
             WHERE started_at<=? ORDER BY revision_id",
        )
        .bind(started_before)
        .fetch_all(&mut *tx)
        .await?;
        for revision_id in &revisions {
            sqlx::query("DELETE FROM publication_freeze_attempts WHERE revision_id=?")
                .bind(revision_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE publication_revisions SET state='draft',updated_at=? \
                 WHERE id=? AND state='freezing'",
            )
            .bind(chrono::Utc::now().timestamp())
            .bind(revision_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(revisions)
    }

    pub async fn commit_publication_freeze(
        &self,
        commit: &PublicationFreezeCommit,
    ) -> Result<PublicationRevision> {
        if commit.revision_id != commit.readiness.revision_id
            || !commit.readiness.can_freeze
            || commit
                .readiness
                .blockers
                .iter()
                .any(|finding| !finding.waived || !finding.waivable)
        {
            anyhow::bail!("Publication readiness contains unresolved blockers");
        }
        let policy_value: serde_json::Value = serde_json::from_str(&commit.policy_json)
            .map_err(|_| anyhow::anyhow!("Publication freeze policy must be valid JSON"))?;
        let canonical_policy = canonical_json(&policy_value);
        if canonical_policy != commit.policy_json {
            anyhow::bail!("Publication freeze policy must be canonical JSON");
        }
        let manifest_value: serde_json::Value =
            serde_json::from_str(&commit.readiness.manifest_json)
                .map_err(|_| anyhow::anyhow!("Publication manifest must be valid JSON"))?;
        let (canonical_manifest, manifest_sha256) = canonical_json_sha256(&manifest_value);
        if canonical_manifest != commit.readiness.manifest_json
            || manifest_sha256 != commit.readiness.manifest_sha256
        {
            anyhow::bail!("Publication manifest hash or canonical form is invalid");
        }
        if manifest_value
            .get("schema_version")
            .and_then(|value| value.as_i64())
            != Some(1)
            || manifest_value
                .get("publication_revision_id")
                .and_then(|value| value.as_str())
                != Some(commit.revision_id.as_str())
            || manifest_value
                .get("target_visibility")
                .and_then(|value| value.as_str())
                != Some(commit.readiness.target_visibility.as_str())
            || manifest_value
                .get("capability_level")
                .and_then(|value| value.as_str())
                != Some(commit.readiness.capability_level.as_str())
            || manifest_value.get("policy") != Some(&policy_value)
            || manifest_value.get("blockers")
                != Some(&serde_json::to_value(&commit.readiness.blockers)?)
            || manifest_value.get("warnings")
                != Some(&serde_json::to_value(&commit.readiness.warnings)?)
            || manifest_value.get("omissions")
                != Some(&serde_json::to_value(&commit.readiness.omissions)?)
        {
            anyhow::bail!("Publication manifest does not match its prepared readiness");
        }

        let now = chrono::Utc::now().timestamp();
        let mut tx = self.begin_write().await?;
        let attempt = sqlx::query(
            "SELECT attempt.revision_id,attempt.target_visibility,attempt.policy_json,\
                    publication.project_id \
             FROM publication_freeze_attempts attempt \
             JOIN publication_revisions revision ON revision.id=attempt.revision_id \
             JOIN publications publication ON publication.id=revision.publication_id \
             WHERE attempt.id=? AND revision.state='freezing'",
        )
        .bind(&commit.attempt_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Publication freeze attempt is no longer active"))?;
        let attempt_revision: String = attempt.try_get("revision_id")?;
        let target_visibility: String = attempt.try_get("target_visibility")?;
        let stored_policy: String = attempt.try_get("policy_json")?;
        let project_id: String = attempt.try_get("project_id")?;
        if attempt_revision != commit.revision_id
            || target_visibility != commit.readiness.target_visibility.as_str()
            || stored_policy != canonical_policy
        {
            anyhow::bail!("Publication freeze attempt no longer matches its prepared policy");
        }

        let mut captures = commit.late_captures.clone();
        captures.sort_by(|left, right| left.new_version_id.cmp(&right.new_version_id));
        for capture in captures {
            if capture.binding_ids.is_empty()
                || capture.version_number <= 0
                || capture.size_bytes < 0
                || capture.checksum.len() != 64
                || !capture
                    .checksum
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
                || matches!(capture.materialization, ArtifactMaterialization::External)
            {
                anyhow::bail!("Prepared late capture is invalid");
            }
            let snapshot_value: serde_json::Value =
                serde_json::from_str(&capture.source_snapshot_json)
                    .map_err(|_| anyhow::anyhow!("Late-capture source snapshot is invalid"))?;
            if canonical_json(&snapshot_value) != capture.source_snapshot_json {
                anyhow::bail!("Late-capture source snapshot must be canonical JSON");
            }
            let artifact =
                sqlx::query("SELECT project_id,latest_version_id FROM artifacts WHERE id=?")
                    .bind(&capture.artifact_id)
                    .fetch_optional(&mut *tx)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Late-capture Artifact no longer exists"))?;
            let artifact_project: String = artifact.try_get("project_id")?;
            let latest_version_id: Option<String> = artifact.try_get("latest_version_id")?;
            if artifact_project != project_id
                || latest_version_id != capture.expected_latest_version_id
            {
                anyhow::bail!("Artifact changed while Publication freeze was prepared");
            }
            let expected_number: i64 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(version_number),0)+1 FROM artifact_versions \
                 WHERE artifact_id=?",
            )
            .bind(&capture.artifact_id)
            .fetch_one(&mut *tx)
            .await?;
            if expected_number != capture.version_number {
                anyhow::bail!("Artifact version sequence changed during Publication freeze");
            }
            for binding_id in &capture.binding_ids {
                let exact_binding: bool = sqlx::query_scalar(
                    "SELECT EXISTS(\
                       SELECT 1 FROM evidence_bindings binding \
                       JOIN artifact_versions version \
                         ON version.id=binding.artifact_version_id \
                       WHERE binding.id=? AND binding.revision_id=? \
                         AND binding.artifact_version_id=? AND version.artifact_id=?\
                     )",
                )
                .bind(binding_id)
                .bind(&commit.revision_id)
                .bind(&capture.old_version_id)
                .bind(&capture.artifact_id)
                .fetch_one(&mut *tx)
                .await?;
                if !exact_binding {
                    anyhow::bail!("Evidence binding changed while Publication freeze was prepared");
                }
            }
            sqlx::query(
                "INSERT INTO artifact_versions(\
                   id,artifact_id,version_number,content_type,storage_path,size_bytes,checksum,\
                   parent_version_id,producing_run_id,env_snapshot_hash,materialization,\
                   capture_timing,created_at\
                 ) VALUES(?,?,?,?,?,?,?,?,NULL,NULL,?,'late',?)",
            )
            .bind(&capture.new_version_id)
            .bind(&capture.artifact_id)
            .bind(capture.version_number)
            .bind(&capture.content_type)
            .bind(&capture.storage_path)
            .bind(capture.size_bytes)
            .bind(&capture.checksum)
            .bind(capture.expected_latest_version_id.as_deref())
            .bind(capture.materialization.as_str())
            .bind(now)
            .execute(&mut *tx)
            .await?;
            let updated_artifact = sqlx::query(
                "UPDATE artifacts SET latest_version_id=?,storage_path=?,content_type=? \
                 WHERE id=? AND latest_version_id IS ?",
            )
            .bind(&capture.new_version_id)
            .bind(&capture.storage_path)
            .bind(&capture.content_type)
            .bind(&capture.artifact_id)
            .bind(capture.expected_latest_version_id.as_deref())
            .execute(&mut *tx)
            .await?;
            if updated_artifact.rows_affected() != 1 {
                anyhow::bail!("Artifact changed while Publication freeze was committed");
            }
            for binding_id in &capture.binding_ids {
                let updated = sqlx::query(
                    "UPDATE evidence_bindings SET source_id=?,artifact_version_id=?,\
                       source_snapshot_json=?,updated_at=? \
                     WHERE id=? AND revision_id=? AND artifact_version_id=?",
                )
                .bind(&capture.new_version_id)
                .bind(&capture.new_version_id)
                .bind(&capture.source_snapshot_json)
                .bind(now)
                .bind(binding_id)
                .bind(&commit.revision_id)
                .bind(&capture.old_version_id)
                .execute(&mut *tx)
                .await?;
                if updated.rows_affected() != 1 {
                    anyhow::bail!(
                        "Evidence binding changed while Publication freeze was committed"
                    );
                }
                let item_id: Option<String> =
                    sqlx::query_scalar("SELECT item_id FROM evidence_bindings WHERE id=?")
                        .bind(binding_id)
                        .fetch_one(&mut *tx)
                        .await?;
                let metadata = canonical_json(&serde_json::json!({
                    "binding_id": binding_id,
                    "revision_id": commit.revision_id,
                    "item_id": item_id,
                    "source_kind": "artifact_version",
                    "source_id": capture.new_version_id,
                }));
                sqlx::query("UPDATE research_edges SET metadata_json=? WHERE id=?")
                    .bind(metadata)
                    .bind(format!("publication-evidence:{binding_id}"))
                    .execute(&mut *tx)
                    .await?;
            }
        }

        let blockers_json = canonical_json(&serde_json::to_value(&commit.readiness.blockers)?);
        let warnings_json = canonical_json(&serde_json::to_value(&commit.readiness.warnings)?);
        let omissions_json = canonical_json(&serde_json::to_value(&commit.readiness.omissions)?);
        sqlx::query(
            "INSERT INTO publication_readiness_reports(\
               id,revision_id,capability_level,target_visibility,policy_json,blockers_json,\
               warnings_json,omissions_json,manifest_json,manifest_sha256,created_at\
             ) VALUES(?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(format!("publication-readiness:{}", commit.revision_id))
        .bind(&commit.revision_id)
        .bind(commit.readiness.capability_level.as_str())
        .bind(commit.readiness.target_visibility.as_str())
        .bind(&canonical_policy)
        .bind(blockers_json)
        .bind(warnings_json)
        .bind(omissions_json)
        .bind(&commit.readiness.manifest_json)
        .bind(&commit.readiness.manifest_sha256)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        let removed_attempt = sqlx::query("DELETE FROM publication_freeze_attempts WHERE id=?")
            .bind(&commit.attempt_id)
            .execute(&mut *tx)
            .await?;
        if removed_attempt.rows_affected() != 1 {
            anyhow::bail!("Publication freeze attempt disappeared before commit");
        }
        let frozen = sqlx::query(
            "UPDATE publication_revisions SET \
               state='frozen',capability_level=?,manifest_json=?,manifest_sha256=?,\
               frozen_at=?,updated_at=? \
             WHERE id=? AND state='freezing'",
        )
        .bind(commit.readiness.capability_level.as_str())
        .bind(&commit.readiness.manifest_json)
        .bind(&commit.readiness.manifest_sha256)
        .bind(now)
        .bind(now)
        .bind(&commit.revision_id)
        .execute(&mut *tx)
        .await?;
        if frozen.rows_affected() != 1 {
            anyhow::bail!("Publication revision changed before freeze commit");
        }
        tx.commit().await?;
        self.get_publication_revision(&commit.revision_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Frozen Publication revision was not persisted"))
    }

    pub async fn get_publication_readiness_report(
        &self,
        revision_id: &str,
    ) -> Result<Option<PublicationReadinessReport>> {
        let row = sqlx::query(
            "SELECT id,revision_id,capability_level,target_visibility,policy_json,\
                    blockers_json,warnings_json,omissions_json,manifest_json,manifest_sha256,\
                    created_at \
             FROM publication_readiness_reports WHERE revision_id=?",
        )
        .bind(revision_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let capability: String = row.try_get("capability_level")?;
            let visibility: String = row.try_get("target_visibility")?;
            Ok(PublicationReadinessReport {
                id: row.try_get("id")?,
                revision_id: row.try_get("revision_id")?,
                capability_level: PublicationCapabilityLevel::from_storage(&capability)?,
                target_visibility: EvidenceVisibility::from_storage(&visibility)?,
                policy_json: row.try_get("policy_json")?,
                blockers_json: row.try_get("blockers_json")?,
                warnings_json: row.try_get("warnings_json")?,
                omissions_json: row.try_get("omissions_json")?,
                manifest_json: row.try_get("manifest_json")?,
                manifest_sha256: row.try_get("manifest_sha256")?,
                created_at: row.try_get("created_at")?,
            })
        })
        .transpose()
    }

    pub async fn list_publication_evidence_drift(
        &self,
        revision_id: &str,
    ) -> Result<Vec<PublicationEvidenceDrift>> {
        let rows = sqlx::query(
            "SELECT binding.id AS binding_id,artifact.id AS artifact_id,\
                    artifact.logical_key AS logical_key,bound.id AS bound_version_id,\
                    bound.version_number AS bound_version_number,\
                    latest.id AS latest_version_id,latest.version_number AS latest_version_number \
             FROM evidence_bindings binding \
             JOIN artifact_versions bound ON bound.id=binding.artifact_version_id \
             JOIN artifacts artifact ON artifact.id=bound.artifact_id \
             JOIN artifact_versions latest ON latest.id=artifact.latest_version_id \
             WHERE binding.revision_id=? AND binding.source_kind='artifact_version' \
             ORDER BY binding.id",
        )
        .bind(revision_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                let bound_version_id: String = row.try_get("bound_version_id")?;
                let latest_version_id: String = row.try_get("latest_version_id")?;
                Ok(PublicationEvidenceDrift {
                    binding_id: row.try_get("binding_id")?,
                    artifact_id: row.try_get("artifact_id")?,
                    logical_key: row.try_get("logical_key")?,
                    has_drift: bound_version_id != latest_version_id,
                    bound_version_id,
                    bound_version_number: row.try_get("bound_version_number")?,
                    latest_version_id,
                    latest_version_number: row.try_get("latest_version_number")?,
                })
            })
            .collect()
    }

    pub async fn publish_publication_revision(&self, revision_id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let updated = sqlx::query(
            "UPDATE publication_revisions SET state='published',published_at=?,updated_at=? \
             WHERE id=? AND state='frozen'",
        )
        .bind(now)
        .bind(now)
        .bind(revision_id)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() != 1 {
            anyhow::bail!("Frozen Publication revision not found");
        }
        Ok(())
    }

    pub async fn clone_publication_revision(
        &self,
        source_revision_id: &str,
        new_revision_id: &str,
        label: &str,
    ) -> Result<PublicationRevision> {
        if new_revision_id.trim().is_empty() || label.trim().is_empty() {
            anyhow::bail!("Cloned revision requires identity and label");
        }
        let now = chrono::Utc::now().timestamp();
        let mut tx = self.begin_write().await?;
        let source: Option<(String, String)> =
            sqlx::query_as("SELECT publication_id,state FROM publication_revisions WHERE id=?")
                .bind(source_revision_id)
                .fetch_optional(&mut *tx)
                .await?;
        let (publication_id, source_state) =
            source.ok_or_else(|| anyhow::anyhow!("Source revision not found"))?;
        if matches!(source_state.as_str(), "freezing" | "deleting") {
            anyhow::bail!("Publication revision cannot be cloned while {source_state}");
        }
        let revision_number: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(revision_number),0)+1 FROM publication_revisions \
             WHERE publication_id=?",
        )
        .bind(&publication_id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO publication_revisions(\
               id,publication_id,parent_revision_id,revision_number,label,state,capability_level,\
               manifest_json,manifest_sha256,frozen_at,published_at,created_at,updated_at\
             ) VALUES(?,?,?,?,?,'draft','archived',NULL,NULL,NULL,NULL,?,?)",
        )
        .bind(new_revision_id)
        .bind(&publication_id)
        .bind(source_revision_id)
        .bind(revision_number)
        .bind(label.trim())
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        let source_items = sqlx::query(
            "SELECT id,parent_item_id,kind,title,content,ordinal,metadata_json,created_at \
             FROM publication_items WHERE revision_id=? ORDER BY created_at,id",
        )
        .bind(source_revision_id)
        .fetch_all(&mut *tx)
        .await?;
        let item_ids = source_items
            .iter()
            .map(|row| {
                Ok((
                    row.try_get::<String, _>("id")?,
                    uuid::Uuid::new_v4().to_string(),
                ))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        for (index, row) in source_items.iter().enumerate() {
            let source_id: String = row.try_get("id")?;
            sqlx::query(
                "INSERT INTO publication_items(\
                   id,revision_id,parent_item_id,kind,title,content,ordinal,metadata_json,\
                   created_at,updated_at\
                 ) VALUES(?,?,NULL,?,?,?,?,?,?,?)",
            )
            .bind(&item_ids[&source_id])
            .bind(new_revision_id)
            .bind(row.try_get::<String, _>("kind")?)
            .bind(row.try_get::<String, _>("title")?)
            .bind(row.try_get::<String, _>("content")?)
            .bind(-i64::try_from(index + 1).unwrap_or(i64::MAX))
            .bind(row.try_get::<String, _>("metadata_json")?)
            .bind(row.try_get::<i64, _>("created_at")?)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
        for row in &source_items {
            let source_id: String = row.try_get("id")?;
            let parent_id: Option<String> = row.try_get("parent_item_id")?;
            if let Some(parent_id) = parent_id {
                let mapped_parent = item_ids
                    .get(&parent_id)
                    .ok_or_else(|| anyhow::anyhow!("Source item parent is missing"))?;
                sqlx::query("UPDATE publication_items SET parent_item_id=? WHERE id=?")
                    .bind(mapped_parent)
                    .bind(&item_ids[&source_id])
                    .execute(&mut *tx)
                    .await?;
            }
        }
        for row in &source_items {
            let source_id: String = row.try_get("id")?;
            sqlx::query("UPDATE publication_items SET ordinal=? WHERE id=?")
                .bind(row.try_get::<i64, _>("ordinal")?)
                .bind(&item_ids[&source_id])
                .execute(&mut *tx)
                .await?;
        }

        let source_links = sqlx::query(
            "SELECT source_item_id,target_item_id,relation,created_at \
             FROM publication_item_links WHERE revision_id=? ORDER BY created_at,id",
        )
        .bind(source_revision_id)
        .fetch_all(&mut *tx)
        .await?;
        for row in source_links {
            let source_item: String = row.try_get("source_item_id")?;
            let target_item: String = row.try_get("target_item_id")?;
            sqlx::query(
                "INSERT INTO publication_item_links(\
                   id,revision_id,source_item_id,target_item_id,relation,created_at\
                 ) VALUES(?,?,?,?,?,?)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(new_revision_id)
            .bind(
                item_ids
                    .get(&source_item)
                    .ok_or_else(|| anyhow::anyhow!("Source item link is incomplete"))?,
            )
            .bind(
                item_ids
                    .get(&target_item)
                    .ok_or_else(|| anyhow::anyhow!("Source item link is incomplete"))?,
            )
            .bind(row.try_get::<String, _>("relation")?)
            .bind(row.try_get::<i64, _>("created_at")?)
            .execute(&mut *tx)
            .await?;
        }

        let source_bindings = sqlx::query(&format!(
            "SELECT {EVIDENCE_BINDING_COLUMNS} FROM evidence_bindings \
             WHERE revision_id=? ORDER BY created_at,id"
        ))
        .bind(source_revision_id)
        .fetch_all(&mut *tx)
        .await?;
        let binding_ids = source_bindings
            .iter()
            .map(|row| {
                Ok((
                    row.try_get::<String, _>("id")?,
                    uuid::Uuid::new_v4().to_string(),
                ))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        for row in &source_bindings {
            let source_id: String = row.try_get("id")?;
            let item_id = row
                .try_get::<Option<String>, _>("item_id")?
                .map(|id| {
                    item_ids
                        .get(&id)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("Evidence item is missing"))
                })
                .transpose()?;
            let claim_id = row
                .try_get::<Option<String>, _>("supported_claim_item_id")?
                .map(|id| {
                    item_ids
                        .get(&id)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("Evidence claim is missing"))
                })
                .transpose()?;
            sqlx::query(
                "INSERT INTO evidence_bindings(\
                   id,revision_id,item_id,source_kind,source_id,artifact_version_id,run_id,\
                   external_resource_id,purpose,supported_claim_item_id,selection_state,\
                   review_state,reproduction_state,visibility,source_snapshot_json,\
                   created_at,updated_at\
                 ) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            )
            .bind(&binding_ids[&source_id])
            .bind(new_revision_id)
            .bind(item_id)
            .bind(row.try_get::<String, _>("source_kind")?)
            .bind(row.try_get::<String, _>("source_id")?)
            .bind(row.try_get::<Option<String>, _>("artifact_version_id")?)
            .bind(row.try_get::<Option<String>, _>("run_id")?)
            .bind(row.try_get::<Option<String>, _>("external_resource_id")?)
            .bind(row.try_get::<String, _>("purpose")?)
            .bind(claim_id)
            .bind(row.try_get::<String, _>("selection_state")?)
            .bind(row.try_get::<String, _>("review_state")?)
            .bind(row.try_get::<String, _>("reproduction_state")?)
            .bind(row.try_get::<String, _>("visibility")?)
            .bind(row.try_get::<String, _>("source_snapshot_json")?)
            .bind(row.try_get::<i64, _>("created_at")?)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }

        let source_reviews = sqlx::query(
            "SELECT review.id,review.binding_id,review.reviewer,review.method,\
                    review.verified_at,review.environment_json,review.comparator_json,\
                    review.tolerance_json,review.result,review.report_json,review.created_at \
             FROM evidence_reviews review \
             JOIN evidence_bindings binding ON binding.id=review.binding_id \
             WHERE binding.revision_id=? ORDER BY review.created_at,review.id",
        )
        .bind(source_revision_id)
        .fetch_all(&mut *tx)
        .await?;
        for row in source_reviews {
            let binding_id: String = row.try_get("binding_id")?;
            sqlx::query(
                "INSERT INTO evidence_reviews(\
                   id,binding_id,reviewer,method,verified_at,environment_json,comparator_json,\
                   tolerance_json,result,report_json,created_at\
                 ) VALUES(?,?,?,?,?,?,?,?,?,?,?)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(
                binding_ids
                    .get(&binding_id)
                    .ok_or_else(|| anyhow::anyhow!("Evidence review binding is missing"))?,
            )
            .bind(row.try_get::<String, _>("reviewer")?)
            .bind(row.try_get::<String, _>("method")?)
            .bind(row.try_get::<i64, _>("verified_at")?)
            .bind(row.try_get::<String, _>("environment_json")?)
            .bind(row.try_get::<String, _>("comparator_json")?)
            .bind(row.try_get::<String, _>("tolerance_json")?)
            .bind(row.try_get::<String, _>("result")?)
            .bind(row.try_get::<String, _>("report_json")?)
            .bind(row.try_get::<i64, _>("created_at")?)
            .execute(&mut *tx)
            .await?;
        }

        let source_supersessions = sqlx::query(
            "SELECT old_binding_id,new_binding_id,reason,created_at \
             FROM evidence_supersessions WHERE revision_id=? ORDER BY created_at,id",
        )
        .bind(source_revision_id)
        .fetch_all(&mut *tx)
        .await?;
        for row in source_supersessions {
            let old_id: String = row.try_get("old_binding_id")?;
            let new_id: String = row.try_get("new_binding_id")?;
            sqlx::query(
                "INSERT INTO evidence_supersessions(\
                   id,revision_id,old_binding_id,new_binding_id,reason,created_at\
                 ) VALUES(?,?,?,?,?,?)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(new_revision_id)
            .bind(
                binding_ids
                    .get(&old_id)
                    .ok_or_else(|| anyhow::anyhow!("Superseded binding is missing"))?,
            )
            .bind(
                binding_ids
                    .get(&new_id)
                    .ok_or_else(|| anyhow::anyhow!("Replacement binding is missing"))?,
            )
            .bind(row.try_get::<String, _>("reason")?)
            .bind(row.try_get::<i64, _>("created_at")?)
            .execute(&mut *tx)
            .await?;
        }

        let source_waivers = sqlx::query(
            "SELECT finding_code,author,reason,created_at \
             FROM publication_waivers WHERE revision_id=? ORDER BY created_at,id",
        )
        .bind(source_revision_id)
        .fetch_all(&mut *tx)
        .await?;
        for row in source_waivers {
            sqlx::query(
                "INSERT INTO publication_waivers(\
                   id,revision_id,finding_code,author,reason,created_at\
                 ) VALUES(?,?,?,?,?,?)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(new_revision_id)
            .bind(row.try_get::<String, _>("finding_code")?)
            .bind(row.try_get::<String, _>("author")?)
            .bind(row.try_get::<String, _>("reason")?)
            .bind(row.try_get::<i64, _>("created_at")?)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        for binding_id in binding_ids.values() {
            self.sync_stored_evidence_projection(binding_id).await?;
        }
        self.get_publication_revision(new_revision_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Cloned revision was not persisted"))
    }

    async fn sync_stored_evidence_projection(&self, binding_id: &str) -> Result<()> {
        let binding = self
            .get_evidence_binding(binding_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Evidence binding not found"))?;
        let project_id: String = sqlx::query_scalar(
            "SELECT publication.project_id \
             FROM publication_revisions revision \
             JOIN publications publication ON publication.id=revision.publication_id \
             WHERE revision.id=?",
        )
        .bind(&binding.revision_id)
        .fetch_one(&self.pool)
        .await?;
        let (target_node_id, target_kind, target_title) = match binding.source_kind {
            EvidenceSourceKind::ArtifactVersion => {
                let row = sqlx::query(
                    "SELECT artifact.id,artifact.filename \
                     FROM artifact_versions version \
                     JOIN artifacts artifact ON artifact.id=version.artifact_id \
                     WHERE version.id=?",
                )
                .bind(&binding.source_id)
                .fetch_one(&self.pool)
                .await?;
                let artifact_id: String = row.try_get("id")?;
                (
                    artifact_node_id(&artifact_id),
                    ResearchNodeKind::Artifact,
                    row.try_get::<String, _>("filename")?,
                )
            }
            EvidenceSourceKind::Run => (
                run_node_id(&binding.source_id),
                ResearchNodeKind::Run,
                sqlx::query_scalar("SELECT title FROM runs WHERE id=?")
                    .bind(&binding.source_id)
                    .fetch_one(&self.pool)
                    .await?,
            ),
            _ => anyhow::bail!("Unsupported evidence projection source"),
        };
        self.sync_evidence_projection(
            &binding.id,
            &project_id,
            &binding.revision_id,
            binding.item_id.as_deref(),
            binding.source_kind,
            &binding.source_id,
            &target_node_id,
            target_kind,
            &target_title,
        )
        .await
    }
}
