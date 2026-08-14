//! Ledger of files this app placed on remote servers (run input staging and
//! transfer uploads), so retracted/replaced/no-longer-referenced files can be
//! found and deleted instead of rotting on the server. Run outputs are not
//! recorded here: the run workdir lifecycle covers them.

use super::Store;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteStagingEntry {
    pub id: String,
    pub project_id: String,
    pub context_id: String,
    /// Run that placed the file (input staging run or transfer run).
    pub run_id: Option<String>,
    /// Absolute or `~/…` path on the remote server.
    pub remote_path: String,
    /// `run_input` or `transfer`.
    pub source: String,
    pub checksum: Option<String>,
    pub size_bytes: Option<i64>,
    pub created_at: i64,
    pub removed_at: Option<i64>,
}

impl RemoteStagingEntry {
    pub fn new(
        project_id: impl Into<String>,
        context_id: impl Into<String>,
        run_id: Option<String>,
        remote_path: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            context_id: context_id.into(),
            run_id,
            remote_path: remote_path.into(),
            source: source.into(),
            checksum: None,
            size_bytes: None,
            created_at: chrono::Utc::now().timestamp(),
            removed_at: None,
        }
    }

    fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty()
            || self.project_id.trim().is_empty()
            || self.context_id.trim().is_empty()
        {
            anyhow::bail!("Remote staging entry requires id, project, and context");
        }
        if self.remote_path.trim().is_empty() || self.remote_path.contains(['\n', '\r', '\0']) {
            anyhow::bail!("Remote staging entry requires a sane remote path");
        }
        if !matches!(self.source.as_str(), "run_input" | "transfer") {
            anyhow::bail!("Remote staging source must be run_input or transfer");
        }
        Ok(())
    }
}

impl Store {
    pub async fn record_remote_staging(&self, entry: &RemoteStagingEntry) -> Result<()> {
        entry.validate()?;
        sqlx::query(
            "INSERT INTO remote_staging(\
                id,project_id,context_id,run_id,remote_path,source,checksum,size_bytes,\
                created_at,removed_at\
             ) VALUES(?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(&entry.id)
        .bind(&entry.project_id)
        .bind(&entry.context_id)
        .bind(entry.run_id.as_deref())
        .bind(&entry.remote_path)
        .bind(&entry.source)
        .bind(entry.checksum.as_deref())
        .bind(entry.size_bytes)
        .bind(entry.created_at)
        .bind(entry.removed_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_remote_staging(
        &self,
        project_id: &str,
        context_id: &str,
        include_removed: bool,
    ) -> Result<Vec<RemoteStagingEntry>> {
        let sql = if include_removed {
            "SELECT id,project_id,context_id,run_id,remote_path,source,checksum,size_bytes,\
             created_at,removed_at FROM remote_staging \
             WHERE project_id=? AND context_id=? ORDER BY created_at,id"
        } else {
            "SELECT id,project_id,context_id,run_id,remote_path,source,checksum,size_bytes,\
             created_at,removed_at FROM remote_staging \
             WHERE project_id=? AND context_id=? AND removed_at IS NULL ORDER BY created_at,id"
        };
        let rows: Vec<(
            String,
            String,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
            Option<i64>,
            i64,
            Option<i64>,
        )> = sqlx::query_as(sql)
            .bind(project_id)
            .bind(context_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    project_id,
                    context_id,
                    run_id,
                    remote_path,
                    source,
                    checksum,
                    size_bytes,
                    created_at,
                    removed_at,
                )| RemoteStagingEntry {
                    id,
                    project_id,
                    context_id,
                    run_id,
                    remote_path,
                    source,
                    checksum,
                    size_bytes,
                    created_at,
                    removed_at,
                },
            )
            .collect())
    }

    pub async fn mark_remote_staging_removed(&self, ids: &[String]) -> Result<u64> {
        let now = chrono::Utc::now().timestamp();
        let mut removed = 0;
        for id in ids {
            let updated = sqlx::query(
                "UPDATE remote_staging SET removed_at=? WHERE id=? AND removed_at IS NULL",
            )
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
            removed += updated.rows_affected();
        }
        Ok(removed)
    }

    /// Workspace cleanup deleted the run's workdir, taking its staged inputs
    /// with it.
    pub async fn mark_remote_staging_removed_for_run(&self, run_id: &str) -> Result<u64> {
        let now = chrono::Utc::now().timestamp();
        let updated = sqlx::query(
            "UPDATE remote_staging SET removed_at=? \
             WHERE run_id=? AND source='run_input' AND removed_at IS NULL",
        )
        .bind(now)
        .bind(run_id)
        .execute(&self.pool)
        .await?;
        Ok(updated.rows_affected())
    }

    /// Disposal audit: External artifact references whose URI targets this
    /// context (`ssh://<alias>/…`), still the head version of their artifact.
    pub async fn count_external_references_on_context(
        &self,
        project_id: &str,
        uri_prefix: &str,
    ) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM artifact_versions v \
             JOIN artifacts a ON a.id=v.artifact_id \
             WHERE a.project_id=? AND v.materialization='external' \
             AND v.storage_path LIKE ? ESCAPE '\\' \
             AND v.id=(SELECT id FROM artifact_versions latest \
                       WHERE latest.artifact_id=v.artifact_id \
                       ORDER BY latest.version_number DESC LIMIT 1)",
        )
        .bind(project_id)
        .bind(format!(
            "{}%",
            uri_prefix.replace('%', "\\%").replace('_', "\\_")
        ))
        .fetch_one(&self.pool)
        .await?)
    }
}
