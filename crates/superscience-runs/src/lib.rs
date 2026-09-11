//! Shared Run control plane used by the desktop shell and the headless CLI.
//!
//! Hosts register the same tools (`run_in_context`, `monitor_run`, `get_run`,
//! `cancel_run`, harvest/cleanup/transfer) against a SQLite [`superscience_store::Store`]
//! and a [`RunManager`]. Waiting is model-free: do not poll with `shell` sleep.

pub mod exploration_isolation;
pub mod harvest;
pub mod mime;
pub mod snapshot_store;
pub mod ssh_guard;
pub mod ssh_hosts;
pub mod ssh_master;
pub mod storage_prefs;

mod run_context;

pub use run_context::remote_files;
pub use run_context::*;

/// Always-on prompt for hosts that expose the Run tools.
pub fn runs_guidance() -> String {
    "## Runs\n\n\
Long-running standalone commands use **run_in_context** (context_id `local`, or an SSH/WSL context when one is selected). \
Short standalone local scripts can run through **shell**. Runs start a fresh process and cannot reuse objects in a persistent Python/R runtime; keep state-dependent analysis in that runtime even when it takes time. \
After submission, call **monitor_run** with the returned run_id. SuperScience waits without extra model calls. \
Do not use shell `sleep`, `Start-Sleep`, `ps`/`kill -0` polling, `nohup`, background `&`, or repeated **get_run**. \
The `shell` tool has a 60s timeout and is the wrong tool for waiting on jobs. \
If `monitor_run` returns `wait_interrupted`, answer the user, then call it again with the same id. Do not resubmit.\n"
        .into()
}

/// Combine the always-on Runs section with any registered remote contexts.
pub async fn cli_compute_section(store: &superscience_store::Store) -> String {
    let mut out = runs_guidance();
    let default = ssh_hosts::stored_default_execution_context(store).await;
    let default_ctx = match default.as_deref() {
        Some(id) => store.get_execution_context(id).await.ok().flatten(),
        None => None,
    };
    if let Ok(contexts) = store.list_execution_contexts().await {
        if let Some(section) = ssh_hosts::render_contexts_section(&contexts, default_ctx.as_ref()) {
            out.push('\n');
            out.push_str(&section);
        }
    }
    out
}

/// Open (or create) the project-local control-plane database and ensure a
/// project row exists so Runs can be submitted against `local`.
pub async fn open_project_store(
    root: &std::path::Path,
    project_id: &str,
    project_name: &str,
) -> anyhow::Result<superscience_store::Store> {
    let override_path = superscience_paths::env_product_or_wisp("WISP_STORE")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let path = match override_path {
        Some(value) => std::path::PathBuf::from(value),
        None => project_store_path(root),
    };
    let store = superscience_store::Store::open(&path).await?;
    store
        .create_project(project_id, project_name, &root.to_string_lossy())
        .await?;
    Ok(store)
}

/// The branded project store, unless only a legacy `.wisp/wisp.sqlite` exists.
///
/// A pre-rebrand CLI project keeps its runs by continuing to open the old file
/// until the new path has been created; a fresh project never gains a `.wisp`.
fn project_store_path(root: &std::path::Path) -> std::path::PathBuf {
    let current = root
        .join(superscience_paths::PROJECT_DIR_NAME)
        .join(superscience_paths::APP_DB_FILE);
    if current.is_file() {
        return current;
    }
    let legacy = root.join(".wisp").join("wisp.sqlite");
    if legacy.is_file() {
        return legacy;
    }
    current
}

#[cfg(test)]
mod tests {
    use super::project_store_path;

    #[test]
    fn project_store_path_brands_fresh_projects_and_follows_legacy_data() {
        let root = std::env::temp_dir().join(format!("wisp-store-path-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let current = root
            .join(superscience_paths::PROJECT_DIR_NAME)
            .join(superscience_paths::APP_DB_FILE);
        let legacy = root.join(".wisp").join("wisp.sqlite");

        // A fresh project uses the branded path and never gains a `.wisp`.
        assert_eq!(project_store_path(&root), current);

        // A pre-rebrand project with only the old file keeps opening it.
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, b"legacy").unwrap();
        assert_eq!(project_store_path(&root), legacy);

        // Once the branded store exists, it wins even beside the legacy file.
        std::fs::create_dir_all(current.parent().unwrap()).unwrap();
        std::fs::write(&current, b"current").unwrap();
        assert_eq!(project_store_path(&root), current);

        let _ = std::fs::remove_dir_all(&root);
    }
}
