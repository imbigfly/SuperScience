//! Remote staging ledger operations: list what this project placed on a
//! server, classify what is still referenced, and delete retracted/replaced
//! files. Only ledgered paths can ever be deleted — never arbitrary input.

use super::{checked_output, ssh_script_command, RunCommandRunner, REMOTE_RPC_TIMEOUT};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RemoteFileState {
    /// Still referenced: its run is active, or a staged input whose workdir
    /// has not been cleaned yet.
    Active,
    /// A newer upload ledgered the same remote path.
    Replaced,
    /// No live reference; safe to delete.
    Orphan,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RemoteFileView {
    pub id: String,
    pub remote_path: String,
    pub source: String,
    pub run_id: Option<String>,
    pub run_status: Option<String>,
    pub size_bytes: Option<i64>,
    pub created_at: i64,
    pub state: RemoteFileState,
}

pub(crate) async fn list_remote_files(
    store: &wisp_store::Store,
    project_id: &str,
    context_id: &str,
) -> Result<Vec<RemoteFileView>, String> {
    let entries = store
        .list_remote_staging(project_id, context_id, false)
        .await
        .map_err(|e| e.to_string())?;
    // Latest ledger entry per remote path wins; older ones are "replaced".
    let mut latest: HashMap<&str, (i64, &str)> = HashMap::new();
    for entry in &entries {
        let candidate = (entry.created_at, entry.id.as_str());
        latest
            .entry(entry.remote_path.as_str())
            .and_modify(|current| {
                if candidate > *current {
                    *current = candidate;
                }
            })
            .or_insert(candidate);
    }
    let mut views = Vec::with_capacity(entries.len());
    for entry in &entries {
        let run = match entry.run_id.as_deref() {
            Some(run_id) => store.get_run(run_id).await.map_err(|e| e.to_string())?,
            None => None,
        };
        let replaced = latest
            .get(entry.remote_path.as_str())
            .is_some_and(|(_, id)| *id != entry.id);
        let active = run.as_ref().is_some_and(|run| {
            !run.status.is_terminal() || (entry.source == "run_input" && run.cleaned_at.is_none())
        });
        let state = if replaced {
            RemoteFileState::Replaced
        } else if active {
            RemoteFileState::Active
        } else {
            RemoteFileState::Orphan
        };
        views.push(RemoteFileView {
            id: entry.id.clone(),
            remote_path: entry.remote_path.clone(),
            source: entry.source.clone(),
            run_id: entry.run_id.clone(),
            run_status: run.map(|run| run.status.as_str().to_string()),
            size_bytes: entry.size_bytes,
            created_at: entry.created_at,
            state,
        });
    }
    Ok(views)
}

fn removal_payload(paths: &[(String, String)]) -> String {
    let mut payload = String::from("set -eu\n");
    for (id, path) in paths {
        payload.push_str(&super::remote_path_assignment(path));
        payload.push('\n');
        payload.push_str("rm -rf \"$path\"\n");
        payload.push_str(&format!("printf '__WISP_RM__:%s\\n' '{id}'\n"));
    }
    payload
}

/// Delete ledgered files from the server. Active entries require `force`
/// (explicit user confirmation). A path that no longer exists on the server
/// still counts as removed — ledger/reality drift resolves toward removal.
pub(crate) async fn remove_remote_files(
    store: &wisp_store::Store,
    runner: &dyn RunCommandRunner,
    project_id: &str,
    context: &wisp_store::ExecutionContext,
    ids: &[String],
    force: bool,
) -> Result<u64, String> {
    if ids.is_empty() {
        return Err("remove_remote_files requires at least one ledger entry id".into());
    }
    let views = list_remote_files(store, project_id, &context.id).await?;
    let mut targets = Vec::new();
    for id in ids {
        let Some(view) = views.iter().find(|view| &view.id == id) else {
            return Err(format!(
                "remote file entry {id} is not ledgered for this project and server"
            ));
        };
        if view.state == RemoteFileState::Active && !force {
            return Err(format!(
                "{} is still referenced by run {}; pass force only with explicit user \
                 confirmation",
                view.remote_path,
                view.run_id.as_deref().unwrap_or("unknown")
            ));
        }
        targets.push((view.id.clone(), view.remote_path.clone()));
    }
    let connection = crate::ssh_hosts::SshConnection::from_execution_context(context)?;
    let output = checked_output(
        "remove remote files",
        runner
            .run(
                ssh_script_command(
                    &connection,
                    "remove remote files",
                    removal_payload(&targets),
                )?,
                REMOTE_RPC_TIMEOUT,
            )
            .await,
    )?;
    let confirmed: Vec<String> = output
        .stdout
        .lines()
        .filter_map(|line| line.strip_prefix("__WISP_RM__:"))
        .map(|id| id.trim().to_string())
        .collect();
    if confirmed.is_empty() {
        return Err("remote file removal did not confirm any deletion".into());
    }
    store
        .mark_remote_staging_removed(&confirmed)
        .await
        .map_err(|e| e.to_string())
}

/// Disposal audit before dropping a server: what would be abandoned.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ContextDisposalReport {
    pub context_id: String,
    /// Registered artifact references (`ssh://alias/…`) that still live only
    /// on this server.
    pub external_references: i64,
    /// Ledgered files not yet removed from the server.
    pub staged_files: i64,
}

pub(crate) async fn context_disposal_report(
    store: &wisp_store::Store,
    project_id: &str,
    context: &wisp_store::ExecutionContext,
) -> Result<ContextDisposalReport, String> {
    let alias = context.id.strip_prefix("ssh:").unwrap_or(&context.id);
    let external_references = store
        .count_external_references_on_context(project_id, &format!("ssh://{alias}/"))
        .await
        .map_err(|e| e.to_string())?;
    let staged_files = store
        .list_remote_staging(project_id, &context.id, false)
        .await
        .map_err(|e| e.to_string())?
        .len() as i64;
    Ok(ContextDisposalReport {
        context_id: context.id.clone(),
        external_references,
        staged_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removal_payload_deletes_only_quoted_ledgered_paths() {
        let payload = removal_payload(&[
            ("id-1".into(), "~/wisp/proj/data/input.fasta".into()),
            ("id-2".into(), "/scratch/proj/big file.bam".into()),
        ]);
        assert!(payload.contains("path=\"$HOME\"/'wisp/proj/data/input.fasta'"));
        assert!(payload.contains("path='/scratch/proj/big file.bam'"));
        assert_eq!(payload.matches("rm -rf \"$path\"").count(), 2);
        assert!(payload.contains("__WISP_RM__:%s\\n' 'id-1'"));
        assert!(payload.contains("__WISP_RM__:%s\\n' 'id-2'"));
    }
}
