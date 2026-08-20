//! Resolve bundled asset directories in dev (repo root) and release (Tauri resources).
//!
//! Product-facing path and env names live here so write/discovery code does not
//! each invent a SuperScience vs upstream Wisp spelling. Tracing targets and
//! test temp names stay with upstream; do not bulk-rename those from this crate.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Project-local directory written by new installs (`history`, `skills`, …).
pub const PROJECT_DIR_NAME: &str = ".superscience";
/// Agent Context file under [`PROJECT_DIR_NAME`].
pub const AGENT_CONTEXT_FILE: &str = "SUPERSCIENCE.md";
/// Compact / agent archive URI scheme (`superscience-history:<id>`).
pub const HISTORY_ARCHIVE_SCHEME: &str = "superscience-history";
/// Documented extra-skills env var. [`LEGACY_SKILLS_PATH_ENV`] is still read.
pub const SKILLS_PATH_ENV: &str = "SUPERSCIENCE_SKILLS_PATH";
/// Silent fallback so upstream scripts that still export this do not break.
pub const LEGACY_SKILLS_PATH_ENV: &str = "WISP_SKILLS_PATH";
/// Documented tool-result budget env var (bytes; `0` disables).
pub const TOOL_RESULT_BUDGET_ENV: &str = "SUPERSCIENCE_TOOL_RESULT_BUDGET";
pub const LEGACY_TOOL_RESULT_BUDGET_ENV: &str = "WISP_TOOL_RESULT_BUDGET";
/// Default HOME-relative remote run workdir root.
pub const DEFAULT_REMOTE_WORKDIR_ROOT: &str = ".superscience/runs";
/// Default remote upload root prefix (`~/superscience/{slug}/data`).
pub const DEFAULT_REMOTE_DATA_ROOT_PREFIX: &str = "~/superscience";
/// Subdirectory under the Tauri app-data dir for the SQLite store and logs.
pub const APP_DATA_DIR_NAME: &str = "superscience";
pub const APP_DB_FILE: &str = "superscience.sqlite";
/// Default Documents workspace folder for a fresh install.
pub const DEFAULT_WORKSPACE_DIR_NAME: &str = "superscience";
/// Windows `%APPDATA%` leaf matching Tauri identifier `science.superscience`.
pub const WINDOWS_APPDATA_ID: &str = "science.superscience";
pub const WINDOWS_LOG_FILE: &str = "superscience.log";
pub const WINDOWS_PREVIOUS_LOG_FILE: &str = "superscience.previous.log";
/// User-visible English product name (settings version, share titles, About).
pub const PRODUCT_NAME: &str = "SuperScience";
pub const PRODUCT_GITHUB: &str = "https://github.com/imbigfly/SuperScience";

/// `<root>/.superscience`
pub fn project_dir(root: &Path) -> PathBuf {
    root.join(PROJECT_DIR_NAME)
}

/// `<root>/.superscience/SUPERSCIENCE.md`
pub fn agent_context_path(root: &Path) -> PathBuf {
    project_dir(root).join(AGENT_CONTEXT_FILE)
}

/// `<root>/.superscience/session.json`
pub fn session_json_path(root: &Path) -> PathBuf {
    project_dir(root).join("session.json")
}

/// `<root>/.superscience/history`
pub fn history_dir(root: &Path) -> PathBuf {
    project_dir(root).join("history")
}

/// `<root>/.superscience/tool-output`
pub fn tool_output_dir(root: &Path) -> PathBuf {
    project_dir(root).join("tool-output")
}

/// `<root>/.superscience/skills`
pub fn project_skills_dir(root: &Path) -> PathBuf {
    project_dir(root).join("skills")
}

/// `<home>/.superscience/skills`
pub fn global_skills_dir(home: &Path) -> PathBuf {
    home.join(PROJECT_DIR_NAME).join("skills")
}

pub fn history_archive_uri(id: &str) -> String {
    format!("{HISTORY_ARCHIVE_SCHEME}:{id}")
}

pub fn default_remote_data_root(project_slug: &str) -> String {
    format!("{DEFAULT_REMOTE_DATA_ROOT_PREFIX}/{project_slug}/data")
}

/// Prefer the documented env var, then the silent upstream fallback.
pub fn env_or_legacy(primary: &str, legacy: &str) -> Option<String> {
    std::env::var(primary)
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var(legacy).ok().filter(|value| !value.is_empty()))
}

/// SuperScience name first (`SUPERSCIENCE_FOO`), then the upstream `WISP_FOO`.
pub fn env_product_or_wisp(wisp_key: &str) -> Option<String> {
    match wisp_key.strip_prefix("WISP_") {
        Some(suffix) => env_or_legacy(&format!("SUPERSCIENCE_{suffix}"), wisp_key),
        None => std::env::var(wisp_key)
            .ok()
            .filter(|value| !value.is_empty()),
    }
}

/// True when either the SuperScience or upstream offline/flag env is set.
pub fn env_flag_product_or_wisp(wisp_key: &str) -> bool {
    match wisp_key.strip_prefix("WISP_") {
        Some(suffix) => {
            std::env::var_os(format!("SUPERSCIENCE_{suffix}")).is_some()
                || std::env::var_os(wisp_key).is_some()
        }
        None => std::env::var_os(wisp_key).is_some(),
    }
}

pub fn tool_result_budget_bytes(default: usize) -> usize {
    env_or_legacy(TOOL_RESULT_BUDGET_ENV, LEGACY_TOOL_RESULT_BUDGET_ENV)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

static RESOURCE_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Set the install resource root (Tauri `resource_dir` in release builds).
pub fn set_resource_root(root: PathBuf) {
    let _ = RESOURCE_ROOT.set(normalize_resource_root(root));
}

/// Prefer the resource layout that actually contains bundled assets.
///
/// Tauri map-form `resources` (current `tauri.conf.json`) place `skills/` at the
/// resource root. Older list-form `../` entries landed under `_up_/`. Some
/// Windows upgrades leave a stale `_up_/` beside a newer top-level tree; always
/// prefer the top-level catalog when it exists so skills like `local-env-setup`
/// are not hidden behind an outdated `_up_/skills`.
pub fn normalize_resource_root(root: PathBuf) -> PathBuf {
    if root.join("skills").is_dir() {
        return root;
    }
    let up = root.join("_up_");
    if up.join("skills").is_dir() {
        up
    } else {
        root
    }
}

fn dev_repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// Root directory containing bundled `skills/`, `python/`, `r/`, etc.
pub fn resource_root() -> PathBuf {
    RESOURCE_ROOT.get().cloned().unwrap_or_else(dev_repo_root)
}

fn existing_dir(base: &Path, rel: &str) -> Option<PathBuf> {
    let p = base.join(rel);
    p.is_dir().then_some(p)
}

pub fn skills_dir() -> Option<PathBuf> {
    existing_dir(&resource_root(), "skills")
}

pub fn python_dir() -> Option<PathBuf> {
    existing_dir(&resource_root(), "python")
}

pub fn r_dir() -> Option<PathBuf> {
    existing_dir(&resource_root(), "r")
}

pub fn bio_tools_dir() -> Option<PathBuf> {
    existing_dir(&resource_root(), "mcp-servers/bio-tools")
}

pub fn seed_dir() -> Option<PathBuf> {
    existing_dir(&resource_root(), "seed")
}

pub fn browser_extension_dir() -> Option<PathBuf> {
    existing_dir(&resource_root(), "browser-extension")
}

pub fn kernel_worker_path() -> Option<PathBuf> {
    python_dir()
        .map(|d| d.join("kernel_worker.py"))
        .filter(|p| p.is_file())
}

pub fn r_kernel_worker_path() -> Option<PathBuf> {
    r_dir()
        .map(|d| d.join("kernel_worker.R"))
        .filter(|p| p.is_file())
}

pub fn mcp_requirements_path() -> Option<PathBuf> {
    python_dir()
        .map(|d| d.join("requirements-mcp.txt"))
        .filter(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_tree_has_bundled_assets() {
        assert!(skills_dir().is_some());
        assert!(python_dir().is_some());
        assert!(r_dir().is_some());
        assert!(r_kernel_worker_path().is_some());
        assert!(bio_tools_dir().is_some());
        assert!(seed_dir().is_some());
        assert!(browser_extension_dir().is_some());
    }

    #[test]
    fn normalize_up_resource_root() {
        let tmp = std::env::temp_dir().join(format!("superscience-paths-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        // Legacy list-form: only `_up_/skills`.
        std::fs::create_dir_all(tmp.join("_up_/skills")).unwrap();
        assert_eq!(normalize_resource_root(tmp.clone()), tmp.join("_up_"));
        // Map-form: top-level `skills/` wins even if a stale `_up_/` remains.
        std::fs::create_dir_all(tmp.join("skills")).unwrap();
        assert_eq!(normalize_resource_root(tmp.clone()), tmp);
        let flat = tmp.join("flat");
        std::fs::create_dir_all(flat.join("skills")).unwrap();
        assert_eq!(normalize_resource_root(flat.clone()), flat);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn product_or_wisp_is_none_when_unset() {
        assert!(env_product_or_wisp("WISP_UNSET_BRAND_PROBE_9f3a").is_none());
        assert!(!env_flag_product_or_wisp("WISP_UNSET_BRAND_PROBE_9f3a"));
    }
}
