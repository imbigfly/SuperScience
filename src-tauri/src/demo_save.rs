//! Save a live session as an Example-project demo (`user-demos/`).

use crate::seed::{self, DemoUiItem};
use crate::session_export::to_workspace_rel;
use crate::AppState;
use chrono::Utc;
use flate2::write::GzEncoder;
use flate2::Compression;
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::{State, WebviewWindow};

const TEXT_FILE_MAX: u64 = 256 * 1024;

#[derive(Serialize)]
pub(super) struct SavedDemo {
    pub id: String,
    pub title: String,
    pub user_saved: bool,
}

#[tauri::command(rename = "save_session_as_demo")]
pub(super) async fn save_session_as_demo_cmd(
    state: State<'_, AppState>,
    window: WebviewWindow,
    session_id: String,
    title: String,
    locale: Option<String>,
    artifact_paths: Option<Vec<String>>,
) -> Result<SavedDemo, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("Demo title is required.".into());
    }
    let title = seed::clean(title);
    let artifact_paths = artifact_paths.unwrap_or_default();
    let messages = state
        .store
        .load_messages(&session_id)
        .await
        .map_err(|e| e.to_string())?;
    if messages.is_empty() {
        return Err("No messages to save.".into());
    }

    let workspace = resolve_workspace(&state, &window, &session_id).await?;
    let stored = state
        .store
        .list_artifacts(&session_id)
        .await
        .unwrap_or_default();
    let replacements = sanitizer_replacements(&workspace);
    let items: Vec<DemoUiItem> = crate::messages_to_items(&messages)
        .into_iter()
        .map(DemoUiItem::from)
        .map(|item| sanitize_item(item, &replacements))
        .collect();
    if items
        .iter()
        .all(|item| item.role != "user" && item.role != "assistant")
    {
        return Err("Session has no conversation to save.".into());
    }

    let request = title.clone();
    let response = items
        .iter()
        .rev()
        .find(|item| item.role == "assistant")
        .map(|item| item.text.clone())
        .unwrap_or_default();
    let thinking = items
        .iter()
        .find(|item| item.role == "reasoning")
        .map(|item| item.text.clone());

    let (workspace_files, binary_files) =
        collect_demo_files(&workspace, &artifact_paths, &stored, &replacements)?;

    let id = unique_demo_id(&seed::user_demos_dir(&state.app_data), &title);
    let dest = seed::user_demos_dir(&state.app_data);
    std::fs::create_dir_all(&dest).map_err(|e| format!("create user-demos: {e}"))?;

    let suffix = id.strip_prefix("manifest_").unwrap_or(&id);
    if !binary_files.is_empty() {
        write_assets_tarball(
            &dest.join(format!("assets_{suffix}.tar.gz")),
            suffix,
            &binary_files,
        )?;
    }

    let locale = normalize_save_locale(locale.as_deref());
    if locale != "en" {
        let overlay_dir = dest.join(locale);
        std::fs::create_dir_all(&overlay_dir).map_err(|e| format!("create overlay dir: {e}"))?;
        let overlay = json!({
            "request": request,
            "response": response,
            "thinking": thinking,
            "items": {},
        });
        write_json(&overlay_dir.join(format!("{id}.i18n.json")), &overlay)?;
    }

    let manifest = json!({
        "root_frame": {
            "id": format!("demo-{suffix}"),
            "parent_frame_id": Value::Null,
            "root_frame_id": format!("demo-{suffix}"),
            "agent_name": "WISP",
            "status": "completed",
            "input_data": { "request": request },
            "output_data": {
                "response": response,
                "thinking": thinking,
                "items": items,
                "workspace_files": workspace_files,
            }
        }
    });
    write_json(&dest.join(format!("{id}.json")), &manifest)?;

    Ok(SavedDemo {
        id,
        title,
        user_saved: true,
    })
}

#[tauri::command(rename = "delete_user_demo")]
pub(super) fn delete_user_demo_cmd(state: State<'_, AppState>, id: String) -> Result<(), String> {
    delete_demo(&state.app_data, &id)
}

#[tauri::command(rename = "export_demo_to_seed")]
pub(super) fn export_demo_to_seed_cmd(
    state: State<'_, AppState>,
    id: String,
    dest_suffix: String,
) -> Result<SavedDemo, String> {
    let dest = seed::bundled_dir().ok_or_else(|| {
        "Bundled example library (seed/) was not found on this install.".to_string()
    })?;
    let saved = export_demo_to_dir(
        &find_demo_dir(&id, Some(&state.app_data))
            .ok_or_else(|| format!("demo '{id}' not found"))?,
        &dest,
        &id,
        &dest_suffix,
    )?;
    seed::unhide_demo(&state.app_data, &saved.id);
    Ok(saved)
}

pub(crate) fn delete_demo(app_data: &Path, id: &str) -> Result<(), String> {
    validate_demo_id(id)?;
    let user_dir = seed::user_demos_dir(app_data);
    if user_dir.join(format!("{id}.json")).is_file() {
        return delete_user_demo_files(&user_dir, id);
    }
    seed::hide_bundled_demo(app_data, id)
}

fn delete_user_demo_files(user_dir: &Path, id: &str) -> Result<(), String> {
    let manifest = user_dir.join(format!("{id}.json"));
    if !manifest.is_file() {
        return Err(format!("user demo '{id}' not found"));
    }
    std::fs::remove_file(&manifest).map_err(|e| format!("delete manifest: {e}"))?;
    let suffix = id.strip_prefix("manifest_").unwrap_or(id);
    let assets = user_dir.join(format!("assets_{suffix}.tar.gz"));
    if assets.is_file() {
        let _ = std::fs::remove_file(assets);
    }
    if let Ok(entries) = std::fs::read_dir(user_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let overlay = path.join(format!("{id}.i18n.json"));
                if overlay.is_file() {
                    let _ = std::fs::remove_file(overlay);
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn bundled_demo_id(suffix: &str) -> Result<String, String> {
    let suffix = suffix
        .trim()
        .trim_start_matches("manifest_")
        .trim_matches('_');
    if suffix.is_empty() || suffix.starts_with("user_") {
        return Err("Invalid bundled example id.".into());
    }
    if suffix.contains("__")
        || !suffix
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(
            "Use lowercase letters, numbers, and underscores, e.g. 01_two_way_anova.".into(),
        );
    }
    Ok(format!("manifest_{suffix}"))
}

pub(crate) fn export_demo_to_dir(
    src_dir: &Path,
    dest_dir: &Path,
    src_id: &str,
    dest_suffix: &str,
) -> Result<SavedDemo, String> {
    validate_demo_id(src_id)?;
    let dest_id = bundled_demo_id(dest_suffix)?;
    let src_manifest = src_dir.join(format!("{src_id}.json"));
    if !src_manifest.is_file() {
        return Err(format!("demo '{src_id}' not found"));
    }
    let dest_manifest = dest_dir.join(format!("{dest_id}.json"));
    if dest_manifest.exists() {
        return Err(format!("bundled demo '{dest_id}' already exists"));
    }
    std::fs::create_dir_all(dest_dir).map_err(|e| format!("create seed dir: {e}"))?;
    std::fs::copy(&src_manifest, &dest_manifest).map_err(|e| format!("copy manifest: {e}"))?;
    let src_suffix = src_id.strip_prefix("manifest_").unwrap_or(src_id);
    let dest_suffix = dest_id.strip_prefix("manifest_").unwrap_or(&dest_id);
    let src_assets = src_dir.join(format!("assets_{src_suffix}.tar.gz"));
    if src_assets.is_file() {
        std::fs::copy(
            &src_assets,
            dest_dir.join(format!("assets_{dest_suffix}.tar.gz")),
        )
        .map_err(|e| format!("copy assets: {e}"))?;
    }
    if let Ok(entries) = std::fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let overlay = path.join(format!("{src_id}.i18n.json"));
            if !overlay.is_file() {
                continue;
            }
            let Some(locale) = path.file_name() else {
                continue;
            };
            let dest_locale = dest_dir.join(locale);
            std::fs::create_dir_all(&dest_locale)
                .map_err(|e| format!("create overlay dir: {e}"))?;
            std::fs::copy(&overlay, dest_locale.join(format!("{dest_id}.i18n.json")))
                .map_err(|e| format!("copy overlay: {e}"))?;
        }
    }
    let title = title_from_manifest(&dest_manifest);
    Ok(SavedDemo {
        id: dest_id,
        title,
        user_saved: false,
    })
}

fn find_demo_dir(id: &str, app_data: Option<&Path>) -> Option<PathBuf> {
    seed::demo_search_dirs(app_data)
        .into_iter()
        .find(|dir| dir.join(format!("{id}.json")).is_file())
}

fn validate_demo_id(id: &str) -> Result<(), String> {
    if id.starts_with("manifest_") && !id.contains('/') && !id.contains('\\') && !id.contains("..")
    {
        Ok(())
    } else {
        Err("Invalid demo id.".into())
    }
}

fn title_from_manifest(path: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(path) else {
        return String::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return String::new();
    };
    let request = value
        .pointer("/root_frame/input_data/request")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim();
    request
        .split(['\n', '。', '.'])
        .next()
        .unwrap_or(request)
        .trim()
        .to_string()
}

async fn resolve_workspace(
    state: &AppState,
    window: &WebviewWindow,
    session_id: &str,
) -> Result<PathBuf, String> {
    if let Some(project_id) = state
        .store
        .frame_project_id(session_id)
        .await
        .map_err(|e| e.to_string())?
    {
        if let Some((_, workspace)) = state
            .store
            .get_project(&project_id)
            .await
            .map_err(|e| e.to_string())?
        {
            if !workspace.trim().is_empty() {
                return Ok(PathBuf::from(workspace));
            }
        }
    }
    Ok(state.active(window.label()).root)
}

fn normalize_save_locale(locale: Option<&str>) -> &'static str {
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

pub(crate) fn demo_slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
        if out.len() >= 32 {
            break;
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "session".into()
    } else {
        out
    }
}

fn unique_demo_id(user_dir: &Path, title: &str) -> String {
    let stamp = Utc::now().format("%Y%m%d_%H%M%S");
    let slug = demo_slug(title);
    let mut id = format!("manifest_user_{stamp}_{slug}");
    let mut n = 2u32;
    while user_dir.join(format!("{id}.json")).is_file() {
        id = format!("manifest_user_{stamp}_{slug}_{n}");
        n += 1;
    }
    id
}

pub(crate) fn sanitizer_replacements(workspace: &Path) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let ws = workspace.to_string_lossy().replace('\\', "/");
    let ws = ws.trim_end_matches('/').to_string();
    if !ws.is_empty() {
        pairs.push((format!("{ws}/"), String::new()));
        pairs.push((ws, String::new()));
    }
    if let Some(home) = dirs::home_dir() {
        let home = home.to_string_lossy().replace('\\', "/");
        let home = home.trim_end_matches('/').to_string();
        if !home.is_empty() {
            pairs.push((format!("{home}/"), "~/".into()));
            pairs.push((home, "~".into()));
        }
    }
    pairs.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    pairs
}

pub(crate) fn sanitize_demo_text(text: &str, replacements: &[(String, String)]) -> String {
    let mut out = seed::clean(text);
    for (from, to) in replacements {
        if from.is_empty() {
            continue;
        }
        out = out.replace(from, to);
        let win = from.replace('/', "\\");
        if win != *from {
            out = out.replace(&win, to);
        }
    }
    static EMAIL: OnceLock<Regex> = OnceLock::new();
    let email = EMAIL
        .get_or_init(|| Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap());
    out = email
        .replace_all(&out, |caps: &regex::Captures| {
            let value = &caps[0];
            if value.ends_with("@example.com") {
                value.to_string()
            } else {
                "demo@example.com".into()
            }
        })
        .into_owned();
    out
}

fn sanitize_item(mut item: DemoUiItem, replacements: &[(String, String)]) -> DemoUiItem {
    item.text = sanitize_demo_text(&item.text, replacements);
    if let Some(input) = item.input.as_mut() {
        *input = sanitize_demo_text(input, replacements);
    }
    item
}

fn collect_demo_files(
    workspace: &Path,
    artifact_paths: &[String],
    stored: &[(String, String, String, String, i64, Option<String>)],
    replacements: &[(String, String)],
) -> Result<(BTreeMap<String, String>, Vec<(PathBuf, String)>), String> {
    let mut candidates = artifact_paths.to_vec();
    candidates.extend(stored.iter().map(|(_, _, _, path, _, _)| path.clone()));
    let mut seen = std::collections::HashSet::<String>::new();
    let mut workspace_files = BTreeMap::new();
    let mut binary_files = Vec::new();
    for source in candidates {
        let real = match superscience_tools::safety::validate_file_path(workspace, &source) {
            Ok(real) => real,
            Err(_) => continue,
        };
        let rel = to_workspace_rel(workspace, &real.to_string_lossy());
        if rel.is_empty()
            || rel.contains(".superscience/artifacts/sha256")
            || !seen.insert(rel.clone())
        {
            continue;
        }
        let meta = match std::fs::metadata(&real) {
            Ok(meta) if meta.is_file() => meta,
            _ => continue,
        };
        if let Ok(text) = std::fs::read_to_string(&real) {
            if meta.len() <= TEXT_FILE_MAX {
                workspace_files.insert(rel.clone(), sanitize_demo_text(&text, replacements));
            }
        }
        if is_binary_name(&rel) || meta.len() > TEXT_FILE_MAX {
            if let Some(name) = Path::new(&rel).file_name().and_then(|n| n.to_str()) {
                binary_files.push((real, name.to_string()));
            }
        }
    }
    Ok((workspace_files, binary_files))
}

fn is_binary_name(rel: &str) -> bool {
    matches!(
        Path::new(rel)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "pdf" | "zip" | "gz" | "tar")
    )
}

fn write_assets_tarball(
    dest: &Path,
    suffix: &str,
    files: &[(PathBuf, String)],
) -> Result<(), String> {
    let file = File::create(dest).map_err(|e| format!("create assets: {e}"))?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(enc);
    let mut used = std::collections::HashSet::<String>::new();
    for (src, name) in files {
        let mut unique = name.clone();
        let mut n = 2u32;
        while !used.insert(unique.clone()) {
            unique = format!("{n}-{name}");
            n += 1;
        }
        builder
            .append_path_with_name(src, format!("example_{suffix}/{unique}"))
            .map_err(|e| format!("pack {}: {e}", src.display()))?;
    }
    builder
        .finish()
        .map_err(|e| format!("finish assets: {e}"))?;
    Ok(())
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let body = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let mut file = File::create(path).map_err(|e| format!("write {}: {e}", path.display()))?;
    file.write_all(body.as_bytes())
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    file.write_all(b"\n")
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn slugs_ascii_and_falls_back_for_cjk() {
        assert_eq!(demo_slug("Run two-way ANOVA"), "run_two_way_anova");
        assert_eq!(demo_slug("【统计建模与检验】"), "session");
    }

    #[test]
    fn sanitizes_home_workspace_and_email() {
        let tmp = std::env::temp_dir().join(format!("ss-sanitize-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let pairs = sanitizer_replacements(&tmp);
        let home = dirs::home_dir().unwrap();
        let raw = format!(
            "Uploaded files: {}/uploads/完整表达矩阵.csv\nContact imbigfly <you@lab.org>\nHome {}",
            tmp.display(),
            home.display()
        );
        let cleaned = sanitize_demo_text(&raw, &pairs);
        assert!(
            !cleaned.contains(&tmp.to_string_lossy().to_string()),
            "{cleaned}"
        );
        assert!(cleaned.contains("uploads/完整表达矩阵.csv"), "{cleaned}");
        assert!(cleaned.contains("demo@example.com"), "{cleaned}");
        assert!(!cleaned.contains("you@lab.org"), "{cleaned}");
        assert!(
            !cleaned.contains(&home.to_string_lossy().to_string()),
            "{cleaned}"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn hides_bundled_demo_ids() {
        let tmp = std::env::temp_dir().join(format!("ss-del-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        delete_demo(&tmp, "manifest_esr1_01_datasets").unwrap();
        assert!(seed::load_hidden_ids(&tmp).contains(&"manifest_esr1_01_datasets".to_string()));
        assert!(delete_demo(&tmp, "manifest_missing_demo").is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn deletes_only_user_manifest_and_assets() {
        let tmp = std::env::temp_dir().join(format!("ss-del-ok-{}", Uuid::new_v4()));
        let user = tmp.join("user-demos");
        std::fs::create_dir_all(user.join("zh")).unwrap();
        let id = "manifest_user_20260818_100000_session";
        std::fs::write(user.join(format!("{id}.json")), "{}").unwrap();
        std::fs::write(user.join("zh").join(format!("{id}.i18n.json")), "{}").unwrap();
        std::fs::write(
            user.join("assets_user_20260818_100000_session.tar.gz"),
            b"x",
        )
        .unwrap();
        delete_demo(&tmp, id).unwrap();
        assert!(!user.join(format!("{id}.json")).exists());
        assert!(!user.join("zh").join(format!("{id}.i18n.json")).exists());
        assert!(!user
            .join("assets_user_20260818_100000_session.tar.gz")
            .exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn bundled_ids_reject_user_prefix_and_bad_chars() {
        assert_eq!(
            bundled_demo_id("01_two_way").unwrap(),
            "manifest_01_two_way"
        );
        assert_eq!(
            bundled_demo_id("manifest_01_two_way").unwrap(),
            "manifest_01_two_way"
        );
        assert!(bundled_demo_id("user_session").is_err());
        assert!(bundled_demo_id("01 Two Way").is_err());
        assert!(bundled_demo_id("").is_err());
    }

    #[test]
    fn export_copies_manifest_assets_and_overlay() {
        let tmp = std::env::temp_dir().join(format!("ss-export-{}", Uuid::new_v4()));
        let src = tmp.join("src");
        let dest = tmp.join("seed");
        std::fs::create_dir_all(src.join("zh")).unwrap();
        let src_id = "manifest_user_20260818_100000_session";
        std::fs::write(
            src.join(format!("{src_id}.json")),
            r#"{"root_frame":{"input_data":{"request":"Tissue age ANOVA."},"output_data":{"response":"done"}}}"#,
        )
        .unwrap();
        std::fs::write(src.join("zh").join(format!("{src_id}.i18n.json")), "{}").unwrap();
        std::fs::write(
            src.join("assets_user_20260818_100000_session.tar.gz"),
            b"gz",
        )
        .unwrap();
        let saved = export_demo_to_dir(&src, &dest, src_id, "01_tissue_age").unwrap();
        assert_eq!(saved.id, "manifest_01_tissue_age");
        assert_eq!(saved.title, "Tissue age ANOVA");
        assert!(!saved.user_saved);
        assert!(dest.join("manifest_01_tissue_age.json").is_file());
        assert!(dest.join("assets_01_tissue_age.tar.gz").is_file());
        assert!(dest
            .join("zh")
            .join("manifest_01_tissue_age.i18n.json")
            .is_file());
        assert!(export_demo_to_dir(&src, &dest, src_id, "01_tissue_age").is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
