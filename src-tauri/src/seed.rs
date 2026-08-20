//! Bundled demo loader — reads the upstream `seed/manifest_*.json` session
//! recordings and presents each as a pre-baked transcript the UI can open.
//! Full operation history lives in `output_data.items` (UiItem-shaped rows).
//! Figure/data files live in paired `assets_*.tar.gz` archives and are extracted
//! into the workspace when a demo is opened.
//!
//! Locale-specific narrative overlays live under `seed/{locale}/manifest_*.i18n.json`
//! and rewrite user/assistant/plan/reasoning text (plus request/response/thinking)
//! while keeping tool I/O from the English base manifests.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use superscience_llm::Message;
use superscience_store::Store;
use tauri::State;
use uuid::Uuid;

use crate::resource_refs;
use crate::AppState;

const MAX_SEED_REPEAT: usize = 200;
const MAX_SEED_PAD: usize = 64;

/// Bundled demo manifests (`seed/`).
pub fn bundled_dir() -> Option<PathBuf> {
    superscience_paths::seed_dir()
}

/// User-saved example transcripts live next to the app database.
pub fn user_demos_dir(app_data: &Path) -> PathBuf {
    app_data.join("user-demos")
}

fn hidden_ids_path(app_data: &Path) -> PathBuf {
    user_demos_dir(app_data).join("hidden_ids.json")
}

pub(crate) fn load_hidden_ids(app_data: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(hidden_ids_path(app_data)) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(&text)
        .unwrap_or_default()
        .into_iter()
        .filter(|id| {
            id.starts_with("manifest_")
                && !id.contains('/')
                && !id.contains('\\')
                && !id.contains("..")
        })
        .collect()
}

pub(crate) fn save_hidden_ids(app_data: &Path, ids: &[String]) -> Result<(), String> {
    let dir = user_demos_dir(app_data);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create user-demos: {e}"))?;
    let body = serde_json::to_string_pretty(ids).map_err(|e| e.to_string())?;
    std::fs::write(hidden_ids_path(app_data), format!("{body}\n"))
        .map_err(|e| format!("write hidden ids: {e}"))
}

pub(crate) fn hide_bundled_demo(app_data: &Path, id: &str) -> Result<(), String> {
    let bundled = bundled_dir().ok_or_else(|| "Bundled example library not found.".to_string())?;
    if !bundled.join(format!("{id}.json")).is_file() {
        return Err(format!("demo '{id}' not found"));
    }
    let mut hidden = load_hidden_ids(app_data);
    if !hidden.iter().any(|existing| existing == id) {
        hidden.push(id.to_string());
        hidden.sort();
        save_hidden_ids(app_data, &hidden)?;
    }
    Ok(())
}

pub(crate) fn unhide_demo(app_data: &Path, id: &str) {
    let mut hidden = load_hidden_ids(app_data);
    let before = hidden.len();
    hidden.retain(|existing| existing != id);
    if hidden.len() != before {
        let _ = save_hidden_ids(app_data, &hidden);
    }
}

pub(crate) fn demo_search_dirs(app_data: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(app_data) = app_data {
        let user = user_demos_dir(app_data);
        if user.is_dir() {
            dirs.push(user);
        }
    }
    if let Some(bundled) = bundled_dir() {
        dirs.push(bundled);
    }
    dirs
}

#[derive(Serialize, Clone)]
pub struct DemoInfo {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub user_saved: bool,
}

/// One transcript row returned to the UI (same shape as session `UiItem`).
#[derive(Serialize, Clone, Deserialize)]
pub struct DemoUiItem {
    pub role: String,
    pub text: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locations: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<resource_refs::UiMessageResource>,
}

#[derive(Serialize, Clone)]
pub struct Demo {
    pub id: String,
    pub title: String,
    pub request: String,
    pub response: String,
    pub thinking: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<DemoUiItem>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct DemoI18nOverlay {
    #[serde(default)]
    request: Option<String>,
    #[serde(default)]
    response: Option<String>,
    #[serde(default)]
    thinking: Option<String>,
    /// Map of item index → translated narrative text.
    #[serde(default)]
    items: HashMap<String, DemoI18nItem>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct DemoI18nItem {
    #[serde(default)]
    text: Option<String>,
}

fn normalize_locale(locale: Option<&str>) -> &'static str {
    match locale.map(str::trim).unwrap_or("") {
        "" => "zh",
        s if s.eq_ignore_ascii_case("zh")
            || s.to_ascii_lowercase().starts_with("zh-")
            || s.to_ascii_lowercase().starts_with("zh_") =>
        {
            "zh"
        }
        _ => "en",
    }
}

#[tauri::command(rename = "list_demos")]
pub(super) fn list_demos_cmd(state: State<'_, AppState>, locale: Option<String>) -> Vec<DemoInfo> {
    list_demos_with_user(locale.as_deref(), Some(&state.app_data))
}

#[tauri::command(rename = "load_demo")]
pub(super) fn load_demo_cmd(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    id: String,
    locale: Option<String>,
) -> Result<Demo, String> {
    let ap = state.active(window.label());
    extract_demo_assets_in(&id, &ap.root, &demo_search_dirs(Some(&state.app_data)))?;
    load_demo_from(&id, locale.as_deref(), Some(&state.app_data))
        .ok_or_else(|| format!("demo '{id}' not found"))
}

#[tauri::command(rename = "copy_demo_to_project")]
pub(super) async fn copy_demo_to_project_cmd(
    state: State<'_, AppState>,
    id: String,
    target_project_id: String,
) -> Result<String, String> {
    let (_, workspace_dir) = state
        .store
        .get_project(&target_project_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Target project not found.".to_string())?;
    if workspace_dir.trim().is_empty() {
        return Err("Target project has no workspace.".into());
    }
    let _activity = state.begin_project_activity(&target_project_id)?;
    let model_id = crate::models::active_profile_id(&state.store).await;
    copy_demo_into_project_from(
        &state.store,
        &id,
        &target_project_id,
        Path::new(&workspace_dir),
        &model_id,
        Some(&state.app_data),
    )
    .await
}

#[derive(Deserialize)]
struct DemoSeedTurn {
    role: String,
    #[serde(default)]
    text: String,
    #[serde(default = "default_seed_repeat")]
    repeat: usize,
    /// Repeat `text` inside one message so a compact seed file still expands
    /// into enough tokens for a real `/compact` to fold the opening answer.
    #[serde(default = "default_seed_repeat")]
    pad: usize,
}

fn default_seed_repeat() -> usize {
    1
}

impl From<crate::UiItem> for DemoUiItem {
    fn from(item: crate::UiItem) -> Self {
        Self {
            role: item.role,
            text: item.text,
            tool_name: item.tool_name,
            ok: item.ok,
            duration_ms: item.duration_ms,
            input: item.input,
            model_name: item.model_name,
            call_id: item.call_id,
            kind: item.kind,
            status: item.status,
            locations: item.locations,
            resources: item.resources,
        }
    }
}

fn seed_item(role: &str, text: &str) -> DemoUiItem {
    DemoUiItem {
        role: role.to_string(),
        text: text.to_string(),
        tool_name: None,
        ok: None,
        duration_ms: None,
        input: None,
        model_name: None,
        call_id: None,
        kind: None,
        status: None,
        locations: None,
        resources: Vec::new(),
    }
}

fn expand_seed_turns(turns: Vec<DemoSeedTurn>) -> Vec<DemoUiItem> {
    let mut out = Vec::new();
    for turn in turns {
        let n = turn.repeat.clamp(1, MAX_SEED_REPEAT);
        let pad = turn.pad.clamp(1, MAX_SEED_PAD);
        let text = turn.text.repeat(pad);
        for _ in 0..n {
            out.push(seed_item(&turn.role, &text));
        }
    }
    out
}

pub(crate) fn clean(text: &str) -> String {
    static IMG: OnceLock<Regex> = OnceLock::new();
    static ART: OnceLock<Regex> = OnceLock::new();
    let img = IMG.get_or_init(|| Regex::new(r"!\[([^\]]*)\]\(\{\{artifact:[^}]+\}\}\)").unwrap());
    let art = ART.get_or_init(|| Regex::new(r"\{\{artifact:[^}]+\}\}").unwrap());
    let s = img.replace_all(text, "[$1 (figure)]").to_string();
    art.replace_all(&s, "(artifact)").to_string()
}

fn title_from_request(req: &str) -> String {
    let first = req
        .split(['.', '。', '!', '！', '?', '？'])
        .next()
        .unwrap_or(req);
    first.trim().chars().take(70).collect()
}

fn read_base_title(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let req = v
        .pointer("/root_frame/input_data/request")
        .and_then(|x| x.as_str())?;
    Some(title_from_request(req))
}

fn overlay_path_in(dir: &Path, locale: &str, id: &str) -> Option<PathBuf> {
    if locale == "en" {
        return None;
    }
    let path = dir.join(locale).join(format!("{id}.i18n.json"));
    path.is_file().then_some(path)
}

fn load_overlay_in(dir: &Path, locale: &str, id: &str) -> Option<DemoI18nOverlay> {
    let path = overlay_path_in(dir, locale, id)?;
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn list_demos_in(dir: &Path, locale: &str, user_saved: bool) -> Vec<DemoInfo> {
    let mut out = vec![];
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let stem = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if !stem.starts_with("manifest_") {
            continue;
        }
        let title = if let Some(overlay) = load_overlay_in(dir, locale, &stem) {
            overlay
                .request
                .as_deref()
                .map(title_from_request)
                .filter(|t| !t.is_empty())
                .or_else(|| read_base_title(&p))
                .unwrap_or_else(|| stem.trim_start_matches("manifest_").to_string())
        } else {
            read_base_title(&p).unwrap_or_else(|| stem.trim_start_matches("manifest_").to_string())
        };
        out.push(DemoInfo {
            id: stem,
            title,
            user_saved,
        });
    }
    out
}

/// Enumerate bundled `seed/` demos (tests and the no-app-data path).
pub fn list_demos(locale: Option<&str>) -> Vec<DemoInfo> {
    list_demos_with_user(locale, None)
}

pub fn list_demos_with_user(locale: Option<&str>, app_data: Option<&Path>) -> Vec<DemoInfo> {
    let locale = normalize_locale(locale);
    let mut user = app_data
        .map(user_demos_dir)
        .filter(|dir| dir.is_dir())
        .map(|dir| list_demos_in(&dir, locale, true))
        .unwrap_or_default();
    // Newest user save first (`manifest_user_YYYYMMDD_HHMMSS_*`).
    user.sort_by(|a, b| b.id.cmp(&a.id));
    let mut bundled = bundled_dir()
        .map(|dir| list_demos_in(&dir, locale, false))
        .unwrap_or_default();
    bundled.sort_by(|a, b| a.id.cmp(&b.id));
    let hidden: HashMap<_, _> = app_data
        .map(load_hidden_ids)
        .unwrap_or_default()
        .into_iter()
        .map(|id| (id, ()))
        .collect();
    user.retain(|demo| !hidden.contains_key(&demo.id));
    let user_ids: HashMap<_, _> = user.iter().map(|d| (d.id.clone(), ())).collect();
    bundled.retain(|demo| !user_ids.contains_key(&demo.id) && !hidden.contains_key(&demo.id));
    user.extend(bundled);
    user
}

fn assets_tarball_in(dir: &Path, id: &str) -> Option<PathBuf> {
    let suffix = id.strip_prefix("manifest_")?;
    let path = dir.join(format!("assets_{suffix}.tar.gz"));
    path.is_file().then_some(path)
}

fn assets_tarball(id: &str) -> Option<PathBuf> {
    demo_search_dirs(None)
        .into_iter()
        .find_map(|dir| assets_tarball_in(&dir, id))
}

/// Extract bundled demo files into `dest` (workspace root), flattening the
/// `example_*` folder inside each tarball so transcript filenames resolve.
/// Demos without an assets archive are a no-op.
pub fn extract_demo_assets(id: &str, dest: &Path) -> Result<(), String> {
    extract_demo_assets_in(id, dest, &demo_search_dirs(None))
}

pub(crate) fn extract_demo_assets_in(
    id: &str,
    dest: &Path,
    dirs: &[PathBuf],
) -> Result<(), String> {
    let Some(tar_path) = dirs.iter().find_map(|dir| assets_tarball_in(dir, id)) else {
        return Ok(());
    };
    std::fs::create_dir_all(dest).map_err(|e| format!("create demo dest: {e}"))?;
    let file = File::open(&tar_path).map_err(|e| format!("open {}: {e}", tar_path.display()))?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    for entry in archive.entries().map_err(|e| format!("read tar: {e}"))? {
        let mut entry = entry.map_err(|e| format!("tar entry: {e}"))?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        let path = entry.path().map_err(|e| format!("tar path: {e}"))?;
        let Some(name) = path.file_name() else {
            continue;
        };
        let out = dest.join(name);
        entry
            .unpack(&out)
            .map_err(|e| format!("unpack {}: {e}", out.display()))?;
    }
    Ok(())
}

fn clean_item(mut item: DemoUiItem) -> DemoUiItem {
    item.text = clean(&item.text);
    if let Some(input) = item.input.as_mut() {
        *input = clean(input);
    }
    item
}

fn apply_overlay(mut demo: Demo, overlay: DemoI18nOverlay) -> Demo {
    if let Some(request) = overlay.request {
        demo.request = clean(&request);
        demo.title = title_from_request(&demo.request);
    }
    if let Some(response) = overlay.response {
        demo.response = clean(&response);
    }
    if let Some(thinking) = overlay.thinking {
        demo.thinking = Some(clean(&thinking));
    }
    for (idx_s, item_overlay) in overlay.items {
        let Ok(idx) = idx_s.parse::<usize>() else {
            continue;
        };
        let Some(item) = demo.items.get_mut(idx) else {
            continue;
        };
        if let Some(text) = item_overlay.text {
            item.text = clean(&text);
        }
    }
    demo
}

pub(crate) struct DemoManifest {
    demo: Demo,
    workspace_files: BTreeMap<String, String>,
    dir: PathBuf,
}

fn load_demo_manifest(id: &str) -> Option<DemoManifest> {
    load_demo_manifest_from(id, None)
}

pub(crate) fn load_demo_manifest_from(id: &str, app_data: Option<&Path>) -> Option<DemoManifest> {
    let (dir, path) = demo_search_dirs(app_data).into_iter().find_map(|dir| {
        let path = dir.join(format!("{id}.json"));
        path.is_file().then_some((dir, path))
    })?;
    let text = std::fs::read_to_string(&path).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let req = v
        .pointer("/root_frame/input_data/request")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let resp = v
        .pointer("/root_frame/output_data/response")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let thinking = v
        .pointer("/root_frame/output_data/thinking")
        .and_then(|x| x.as_str())
        .map(String::from);
    let mut items = v
        .pointer("/root_frame/output_data/context_seed")
        .and_then(|x| serde_json::from_value::<Vec<DemoSeedTurn>>(x.clone()).ok())
        .map(expand_seed_turns)
        .unwrap_or_default();
    items.extend(
        v.pointer("/root_frame/output_data/items")
            .and_then(|x| serde_json::from_value::<Vec<DemoUiItem>>(x.clone()).ok())
            .unwrap_or_default(),
    );
    let items = items.into_iter().map(clean_item).collect();
    let workspace_files = v
        .pointer("/root_frame/output_data/workspace_files")
        .and_then(|x| serde_json::from_value::<BTreeMap<String, String>>(x.clone()).ok())
        .unwrap_or_default();
    let title =
        read_base_title(&path).unwrap_or_else(|| id.trim_start_matches("manifest_").to_string());
    Some(DemoManifest {
        demo: Demo {
            id: id.to_string(),
            title,
            request: clean(&req),
            response: clean(&resp),
            thinking: thinking.map(|t| clean(&t)),
            items,
        },
        workspace_files,
        dir,
    })
}

/// Load one demo by id (the manifest file stem, e.g. `manifest_esr1_03_rnaseq`).
pub fn load_demo(id: &str, locale: Option<&str>) -> Option<Demo> {
    load_demo_from(id, locale, None)
}

pub fn load_demo_from(id: &str, locale: Option<&str>, app_data: Option<&Path>) -> Option<Demo> {
    let locale = normalize_locale(locale);
    let mut manifest = load_demo_manifest_from(id, app_data)?;
    if let Some(overlay) = load_overlay_in(&manifest.dir, locale, id) {
        manifest.demo = apply_overlay(manifest.demo, overlay);
    }
    Some(manifest.demo)
}

fn safe_workspace_path(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let path = Path::new(rel);
    if path.is_absolute() {
        return Err(format!("unsafe demo path: {rel}"));
    }
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(seg) => out.push(seg),
            _ => return Err(format!("unsafe demo path: {rel}")),
        }
    }
    if out.as_os_str().is_empty() {
        return Err(format!("unsafe demo path: {rel}"));
    }
    Ok(root.join(out))
}

fn write_workspace_files(root: &Path, files: &BTreeMap<String, String>) -> Result<(), String> {
    for (rel, content) in files {
        let dest = safe_workspace_path(root, rel)?;
        if dest.exists() {
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        std::fs::write(&dest, content).map_err(|e| format!("write {}: {e}", dest.display()))?;
    }
    Ok(())
}

fn demo_items_to_messages(items: &[DemoUiItem]) -> Vec<Message> {
    let mut out = Vec::new();
    let mut pending_reasoning = None;
    for (idx, item) in items.iter().enumerate() {
        match item.role.as_str() {
            "reasoning" => pending_reasoning = Some(item.text.clone()),
            "user" => {
                pending_reasoning = None;
                out.push(Message::user(&item.text));
            }
            "assistant" => {
                let mut message = Message::assistant(&item.text);
                message.reasoning = pending_reasoning.take();
                message.model_name = item.model_name.clone();
                out.push(message);
            }
            "tool" => {
                let name = item.tool_name.clone().unwrap_or_else(|| "tool".to_string());
                let call_id = item
                    .call_id
                    .clone()
                    .unwrap_or_else(|| format!("demo-tool-{idx}"));
                out.push(Message::tool(call_id, name, &item.text));
            }
            _ => {}
        }
    }
    out
}

pub async fn copy_demo_into_project(
    store: &Store,
    demo_id: &str,
    project_id: &str,
    workspace: &Path,
    model_id: &str,
) -> Result<String, String> {
    copy_demo_into_project_from(store, demo_id, project_id, workspace, model_id, None).await
}

pub async fn copy_demo_into_project_from(
    store: &Store,
    demo_id: &str,
    project_id: &str,
    workspace: &Path,
    model_id: &str,
    app_data: Option<&Path>,
) -> Result<String, String> {
    let manifest = load_demo_manifest_from(demo_id, app_data)
        .ok_or_else(|| format!("demo '{demo_id}' not found"))?;
    let messages = demo_items_to_messages(&manifest.demo.items);
    if messages.is_empty() {
        return Err(format!("demo '{demo_id}' has no conversation to copy"));
    }
    write_workspace_files(workspace, &manifest.workspace_files)?;
    extract_demo_assets_in(demo_id, workspace, &demo_search_dirs(app_data))?;
    let frame_id = Uuid::new_v4().to_string();
    store
        .create_frame(&frame_id, project_id, "OPERON", model_id)
        .await
        .map_err(|e| e.to_string())?;
    store
        .replace_messages(&frame_id, &messages)
        .await
        .map_err(|e| e.to_string())?;
    let title = manifest.demo.title.trim();
    if !title.is_empty() {
        let _ = store.rename_session(&frame_id, project_id, title).await;
    }
    Ok(frame_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_esr1_demo_assets() {
        let tmp = std::env::temp_dir().join(format!("superscience-seed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        extract_demo_assets("manifest_01_stats_two_way", &tmp).expect("extract stats assets");
        assert!(tmp.join("expression_matrix.csv").is_file());
        assert!(tmp.join("fig1_eda.png").is_file());

        extract_demo_assets("manifest_esr1_03_rnaseq", &tmp).expect("extract rnaseq assets");
        assert!(tmp.join("GSE153250_counts_matrix.tsv").is_file());
        assert!(tmp.join("GSE153250_sample_groups.txt").is_file());
        assert!(tmp.join("GSE153250_featureCounts_summary.txt").is_file());

        let down = tmp.join("downstream");
        std::fs::create_dir_all(&down).unwrap();
        extract_demo_assets("manifest_esr1_04_downstream", &down)
            .expect("extract downstream assets");
        assert!(down.join("DESeq2_top200.csv").is_file());
        assert!(down.join("research_projects.md").is_file());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn user_saved_demos_list_first() {
        let tmp = std::env::temp_dir().join(format!("superscience-user-demos-{}", Uuid::new_v4()));
        let user = tmp.join("user-demos");
        std::fs::create_dir_all(&user).unwrap();
        std::fs::write(
            user.join("manifest_user_20260818_100000_session.json"),
            r#"{"root_frame":{"input_data":{"request":"Saved two-way ANOVA example."},"output_data":{"response":"done","items":[]}}}"#,
        )
        .unwrap();
        let demos = list_demos_with_user(Some("en"), Some(&tmp));
        assert_eq!(demos[0].id, "manifest_user_20260818_100000_session");
        assert!(demos[0].user_saved);
        assert!(demos
            .iter()
            .any(|d| d.id == "manifest_01_stats_two_way" && !d.user_saved));
        hide_bundled_demo(&tmp, "manifest_01_stats_two_way").unwrap();
        let demos = list_demos_with_user(Some("en"), Some(&tmp));
        assert!(!demos.iter().any(|d| d.id == "manifest_01_stats_two_way"));
        assert!(demos[0].user_saved);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn lists_and_loads_bundled_demos() {
        let demos = list_demos(Some("en"));
        assert_eq!(
            demos.len(),
            7,
            "bundled seed should ship the stats two-way demo, five ESR1 demos, and the long-context memory demo"
        );
        assert_eq!(
            demos.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
            [
                "manifest_01_stats_two_way",
                "manifest_esr1_01_datasets",
                "manifest_esr1_02_samples",
                "manifest_esr1_03_rnaseq",
                "manifest_esr1_04_downstream",
                "manifest_esr1_05_hypotheses",
                "manifest_memory_01_long_context",
            ]
        );
        for info in &demos {
            let demo = load_demo(&info.id, Some("en")).expect("load demo");
            assert!(!demo.request.is_empty());
            assert!(!demo.request.contains("English reply"));
            assert!(!demo.request.to_ascii_lowercase().contains("guotosky"));
            assert!(
                !demo.items.is_empty(),
                "{} should ship transcript items",
                info.id
            );
            let is_esr1 = info.id.starts_with("manifest_esr1_");
            if is_esr1 {
                assert!(
                    demo.items.iter().any(|i| i.role == "tool"),
                    "{} should include tool operation records",
                    info.id
                );
            }
            let blob = serde_json::to_string(&demo).unwrap();
            assert!(!blob.to_ascii_lowercase().contains("guotosky"));
            assert!(!blob.contains("10.10.10."));
            assert!(!blob.contains(":7897"));
            assert!(!blob.to_ascii_lowercase().contains("proxy configured"));
            assert!(!blob.to_ascii_lowercase().contains("proxy settings"));
            assert!(!blob.to_ascii_lowercase().contains("bashrc"));
            assert!(!blob.contains("kimi-k3"));
            assert!(!blob.contains("{{artifact:"));
            if is_esr1 {
                assert!(
                    demo.items
                        .iter()
                        .filter_map(|i| i.model_name.as_deref())
                        .all(|m| m == "deepseek-v4-pro"),
                    "{} should use deepseek-v4-pro for all model labels",
                    info.id
                );
            }
        }

        let stats = load_demo("manifest_01_stats_two_way", Some("en")).expect("stats demo");
        assert!(
            stats.request.contains("two-way ANOVA") || stats.request.contains("tissue"),
            "stats demo request should mention the two-way ANOVA / tissue design"
        );
        assert!(
            stats.items.iter().any(|i| i.role == "tool"),
            "stats demo should include tool operation records"
        );
        assert!(
            stats
                .items
                .iter()
                .any(|i| i.role == "user" && i.text.contains("完整表达矩阵")),
            "stats demo should keep the uploaded expression-matrix turn"
        );
        assert!(
            stats.response.contains("200") && stats.response.contains("双向方差分析"),
            "stats demo response should report the two-way ANOVA result"
        );

        let datasets = load_demo("manifest_esr1_01_datasets", Some("en")).expect("datasets demo");
        assert!(
            datasets.request.contains("MCF7") || datasets.request.contains("ESR1"),
            "datasets demo request should mention ESR1/MCF7"
        );

        let samples = load_demo("manifest_esr1_02_samples", Some("en")).expect("samples demo");
        assert!(
            samples.request.contains("GSE153250"),
            "samples demo request should mention GSE153250"
        );

        let rnaseq = load_demo("manifest_esr1_03_rnaseq", Some("en")).expect("rnaseq demo");
        assert!(
            rnaseq
                .items
                .iter()
                .any(|i| i.tool_name.as_deref() == Some("monitor_run")),
            "rnaseq demo should include SSH/run monitor cards"
        );
        assert!(
            rnaseq.response.contains("GSE153250") || rnaseq.response.contains("siESR1"),
            "rnaseq response should mention the study"
        );

        let downstream =
            load_demo("manifest_esr1_04_downstream", Some("en")).expect("downstream demo");
        assert!(
            downstream.request.contains("differential")
                || downstream.request.contains("GSEA")
                || downstream.request.contains("Enrichr"),
            "downstream demo request should mention enrichment/DEG"
        );

        let hypotheses =
            load_demo("manifest_esr1_05_hypotheses", Some("en")).expect("hypotheses demo");
        assert!(
            hypotheses.request.contains("research projects")
                || hypotheses.request.contains("scientific"),
            "hypotheses demo request should ask for research projects"
        );

        let memory = load_demo("manifest_memory_01_long_context", Some("en")).expect("memory demo");
        assert!(
            memory.items.len() > 100,
            "memory demo should expand into a long transcript, got {}",
            memory.items.len()
        );
        assert_eq!(memory.items[0].role, "user");
        assert!(memory.items[0].text.contains("GSE153250"));
        assert_eq!(memory.items[1].role, "assistant");
        assert!(memory.items[1].text.contains("GENE_FILTER="));
        assert!(memory.items[1].text.contains("PRIMARY_CONTRAST="));
        assert!(memory.items[1].text.contains("FDR_CUTOFF=0.05"));
        // The recorded conversation legitimately reuses the locked values when
        // applying them (turn 2) and in the proposed memory note (turn 6), so
        // they are not exclusive to the opening turn. What must hold: the
        // exact GENE_FILTER phrasing stays out of the protected recent tail,
        // so a full post-compact recall has to come from the checkpoint.
        let gene_filter = "GENE_FILTER=keep genes with CPM > 1 in at least 6 samples";
        assert!(memory.items[1].text.contains(gene_filter));
        for item in &memory.items[memory.items.len().saturating_sub(20)..] {
            assert!(
                !item.text.contains(gene_filter),
                "the exact GENE_FILTER phrasing must not survive in the recent tail"
            );
        }
        let last_user = memory
            .items
            .iter()
            .rev()
            .find(|item| item.role == "user")
            .expect("a user item");
        assert!(last_user
            .text
            .contains("do not restate the opening locked decision"));
        let chars: usize = memory.items.iter().map(|item| item.text.len()).sum();
        assert!(
            chars > 300_000,
            "expanded transcript should carry the full recorded session, got {chars} chars"
        );
        let tokens: usize = demo_items_to_messages(&memory.items)
            .iter()
            .map(superscience_core::ContextManager::estimated_tokens)
            .sum();
        // The transcript is a real recorded session (~104K estimated tokens,
        // ~70K after safe pruning), so a manual /compact installs the semantic
        // checkpoint once the configured window is ~110K or smaller (the fold
        // gate is 60% of the window).
        assert!(
            tokens > 80_000,
            "estimated tokens should exceed a ~110K-class 60% fold gate, got {tokens}"
        );
        assert!(memory.request.contains("Long-context memory demo"));
    }

    #[tokio::test]
    async fn copies_long_context_demo_into_a_project_workspace() {
        let tmp = std::env::temp_dir().join(format!("wisp-copy-demo-{}", Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let store = Store::open(&tmp.join("wisp.sqlite")).await.unwrap();
        let workspace = tmp.join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        store
            .create_project("p", "Demo Target", workspace.to_str().unwrap())
            .await
            .unwrap();

        let session_id = copy_demo_into_project(
            &store,
            "manifest_memory_01_long_context",
            "p",
            &workspace,
            "test-model",
        )
        .await
        .expect("copy demo");

        let messages = store.load_messages(&session_id).await.unwrap();
        assert!(messages.len() > 100);
        assert!(messages[0].content.as_text().contains("GSE153250"));
        assert!(messages[1].content.as_text().contains("GENE_FILTER="));
        assert!(messages[1].content.as_text().contains("FDR_CUTOFF=0.05"));
        let tokens: usize = messages
            .iter()
            .map(superscience_core::ContextManager::estimated_tokens)
            .sum();
        assert!(
            tokens > 80_000,
            "copied session too short to exercise a real fold: {tokens}"
        );
        assert!(workspace
            .join(".superscience/memory/2026-08-13.md")
            .is_file());
        assert!(workspace
            .join(".superscience/memory/2025-05-20.md")
            .is_file());
        assert!(workspace.join("AGENTS.md").is_file());
        assert!(workspace.join(".superscience/SUPERSCIENCE.md").is_file());
        let sessions = store.list_sessions("p").await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions[0].1.contains("Long-context memory demo"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn copies_stats_two_way_demo_into_a_project_workspace() {
        let tmp = std::env::temp_dir().join(format!("wisp-copy-stats-{}", Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let store = Store::open(&tmp.join("wisp.sqlite")).await.unwrap();
        let workspace = tmp.join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        store
            .create_project("p", "Demo Target", workspace.to_str().unwrap())
            .await
            .unwrap();

        let session_id = copy_demo_into_project(
            &store,
            "manifest_01_stats_two_way",
            "p",
            &workspace,
            "test-model",
        )
        .await
        .expect("copy stats demo");

        let messages = store.load_messages(&session_id).await.unwrap();
        assert!(messages
            .iter()
            .any(|m| m.content.as_text().contains("完整表达矩阵")));
        assert!(workspace.join("uploads/完整表达矩阵.csv").is_file());
        assert!(workspace
            .join("deg-two-way/input/expression_matrix.csv")
            .is_file());
        assert!(workspace.join("fig1_eda.png").is_file());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rejects_unsafe_demo_workspace_paths() {
        let root = PathBuf::from("/tmp/wisp-demo-root");
        assert!(safe_workspace_path(&root, "../etc/passwd").is_err());
        assert!(safe_workspace_path(&root, "/etc/passwd").is_err());
        assert!(safe_workspace_path(&root, "").is_err());
        assert_eq!(
            safe_workspace_path(&root, ".wisp/memory/note.md").unwrap(),
            root.join(".wisp/memory/note.md")
        );
    }

    #[test]
    fn chinese_overlay_rewrites_titles_and_request() {
        let en = list_demos(Some("en"));
        let zh = list_demos(Some("zh"));
        assert_eq!(zh.len(), 7);
        assert_eq!(en.len(), 7);

        assert_eq!(zh[0].id, "manifest_01_stats_two_way");
        let zh_stats = zh
            .iter()
            .find(|d| d.id == "manifest_01_stats_two_way")
            .expect("zh stats");
        assert!(
            zh_stats
                .title
                .chars()
                .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
            "zh stats title should contain Chinese characters, got {:?}",
            zh_stats.title
        );

        let en_first = en
            .iter()
            .find(|d| d.id == "manifest_esr1_01_datasets")
            .expect("en datasets");
        let zh_first = zh
            .iter()
            .find(|d| d.id == "manifest_esr1_01_datasets")
            .expect("zh datasets");
        assert_ne!(
            en_first.title, zh_first.title,
            "zh overlay should localize the sidebar title"
        );
        assert!(
            zh_first
                .title
                .chars()
                .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
            "zh title should contain Chinese characters, got {:?}",
            zh_first.title
        );

        let demo = load_demo("manifest_esr1_01_datasets", Some("zh")).expect("load zh demo");
        assert!(
            demo.request
                .chars()
                .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
            "zh request should be Chinese, got {:?}",
            demo.request.chars().take(80).collect::<String>()
        );
        let user = demo
            .items
            .iter()
            .find(|i| i.role == "user")
            .expect("user row");
        assert!(
            user.text
                .chars()
                .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
            "zh user message should be Chinese"
        );
        // Tool rows remain from the English base.
        assert!(demo.items.iter().any(|i| i.role == "tool"));
    }
}
