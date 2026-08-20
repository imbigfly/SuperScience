use super::{
    clear_idle_agents, effective_enabled_skill_names, load_enabled_skill_names, load_skill_index,
    load_skill_tags, normalize_tags, project_skill_catalog, save_enabled_skill_names,
    save_skill_tags, skill_infos, AppState, SkillInfo,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State, WebviewWindow};

async fn list_skill_infos_for_project(state: &AppState, label: &str) -> Vec<SkillInfo> {
    let ap = state.active(label);
    let tags = load_skill_tags(&state.store).await;
    let (all, enabled) = project_skill_catalog(&state.store, &ap).await;
    let plugin_roots = crate::plugins::enabled_plugin_manifests(&state.store, &ap.id)
        .await
        .into_iter()
        .flat_map(|(installation, manifest)| {
            let display_name = manifest.display_name.clone();
            manifest
                .skill_paths(Path::new(&installation.install_root))
                .into_iter()
                .map(move |path| (path, display_name.clone()))
        })
        .collect::<Vec<_>>();
    let mut infos = skill_infos(&all, &tags, enabled.as_ref());
    for info in &mut infos {
        if let Some((_, display_name)) = plugin_roots
            .iter()
            .find(|(root, _)| Path::new(&info.dir).starts_with(root))
        {
            // Managed by its parent plugin; removal happens from the plugin
            // card so files and project bindings stay consistent.
            info.builtin = true;
            info.managed = true;
            info.managed_by = Some(display_name.clone());
        }
    }
    infos
}

#[tauri::command]
pub(super) async fn list_skills(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<Vec<SkillInfo>, String> {
    Ok(list_skill_infos_for_project(&state, window.label()).await)
}

#[tauri::command]
pub(super) async fn reload_skills(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<Vec<SkillInfo>, String> {
    let label = window.label();
    let mut project = state.active(label);
    let previous_names = project
        .skills
        .all()
        .iter()
        .map(|skill| skill.name.clone())
        .collect::<HashSet<_>>();
    let mut enabled = effective_enabled_skill_names(&state.store, &project).await;
    project.skills = Arc::new(load_skill_index(&project.root));

    enabled = enabled_names_after_reload(
        enabled,
        &previous_names,
        project.skills.all().iter().map(|skill| skill.name.as_str()),
    );
    if let Some(names) = &enabled {
        save_enabled_skill_names(&state.store, &project.id, names).await?;
    }

    state.set_active(label, project);
    clear_idle_agents(&state).await;
    Ok(list_skill_infos_for_project(&state, label).await)
}

fn enabled_names_after_reload<'a>(
    enabled: Option<HashSet<String>>,
    previous_names: &HashSet<String>,
    discovered_names: impl IntoIterator<Item = &'a str>,
) -> Option<HashSet<String>> {
    enabled.map(|mut names| {
        names.extend(
            discovered_names
                .into_iter()
                .filter(|name| !previous_names.contains(*name))
                .map(str::to_string),
        );
        names
    })
}

#[tauri::command]
pub(super) async fn set_skill_tags(
    state: State<'_, AppState>,
    name: String,
    tags: Vec<String>,
) -> Result<(), String> {
    let mut all_tags = load_skill_tags(&state.store).await;
    let tags = normalize_tags(tags);
    if tags.is_empty() {
        all_tags.remove(&name);
    } else {
        all_tags.insert(name, tags);
    }
    save_skill_tags(&state.store, &all_tags).await?;
    clear_idle_agents(&state).await;
    Ok(())
}

async fn update_skills_enabled(
    state: &AppState,
    label: &str,
    names: Vec<String>,
    enabled: bool,
) -> Result<(), String> {
    let ap = state.active(label);
    let mut current = effective_enabled_skill_names(&state.store, &ap)
        .await
        .unwrap_or_else(|| ap.skills.all().iter().map(|s| s.name.clone()).collect());
    let known = ap
        .skills
        .all()
        .iter()
        .map(|s| s.name.as_str())
        .collect::<HashSet<_>>();
    for name in names
        .into_iter()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty() && known.contains(n.as_str()))
    {
        if enabled {
            current.insert(name);
        } else {
            current.remove(&name);
        }
    }
    save_enabled_skill_names(&state.store, &ap.id, &current).await?;
    clear_idle_agents(state).await;
    Ok(())
}

#[tauri::command]
pub(super) async fn set_skill_enabled(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    update_skills_enabled(&state, window.label(), vec![name], enabled).await
}

#[tauri::command]
pub(super) async fn set_skills_enabled(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    names: Vec<String>,
    enabled: bool,
) -> Result<(), String> {
    update_skills_enabled(&state, window.label(), names, enabled).await
}

#[tauri::command]
pub(super) async fn pick_skill_source(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    // Folder picking is offered via a second button in the UI.
    app.dialog()
        .file()
        .add_filter("SuperScience skill", &["md", "zip"])
        .pick_file(move |p| {
            let _ = tx.send(p);
        });
    let picked = rx.await.map_err(|e| format!("{e}"))?;
    Ok(picked.map(|fp| fp.to_string()))
}

pub(super) fn user_skills_dir() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|h| h.join(".superscience").join("skills"))
        .ok_or_else(|| "no home directory".to_string())
}

/// Reject skill names that could escape the skills directory. A valid name is a
/// single path component: no separators, no `..`, non-empty.
pub(super) fn validate_skill_name(name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("skill name is empty".into());
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(format!("invalid skill name '{name}'"));
    }
    // Must be exactly one path component (defends against platform-specific tricks).
    if std::path::Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        != Some(name)
    {
        return Err(format!("invalid skill name '{name}'"));
    }
    Ok(())
}

#[tauri::command]
pub(super) async fn install_skill(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    src_path: String,
) -> Result<String, String> {
    let src = PathBuf::from(&src_path);
    let skills_dir = user_skills_dir()?;
    // ZIP extraction, recursive copy, and the atomic directory swap run off
    // the async runtime. Existing user-added skills are replaced so importing
    // an updated package is a normal upgrade path.
    let skill_name = tokio::task::spawn_blocking(move || install_skill_source(&src, &skills_dir))
        .await
        .map_err(|e| format!("{e}"))??;
    reload_host_skill_index(&state, window.label());
    let ap = state.active(window.label());
    if let Some(mut enabled) = load_enabled_skill_names(&state.store, &ap.id).await {
        enabled.insert(skill_name.clone());
        save_enabled_skill_names(&state.store, &ap.id, &enabled).await?;
    }
    clear_idle_agents(&state).await;
    Ok(skill_name)
}

struct SkillImportTempDir(PathBuf);

impl Drop for SkillImportTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(super) fn install_skill_source(src: &Path, skills_dir: &Path) -> Result<String, String> {
    let (_temp_dir, skill_dir, skill_md) = if src.is_dir() {
        let md = src.join("SKILL.md");
        if !md.is_file() {
            return Err("selected folder has no SKILL.md".into());
        }
        (None, src.to_path_buf(), md)
    } else if src.file_name().is_some_and(|name| name == "SKILL.md") {
        (
            None,
            src.parent().map(PathBuf::from).unwrap_or_default(),
            src.to_path_buf(),
        )
    } else if src
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        let temp_root = std::env::temp_dir().join(format!(
            "superscience-skill-import-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&temp_root)
            .map_err(|error| format!("create skill ZIP staging directory: {error}"))?;
        let temp_dir = SkillImportTempDir(temp_root.clone());
        let fallback = src
            .file_stem()
            .and_then(|name| name.to_str())
            .filter(|name| validate_skill_name(name).is_ok())
            .unwrap_or("skill");
        let unpacked = temp_root.join(fallback);
        std::fs::create_dir(&unpacked)
            .map_err(|error| format!("create skill ZIP extraction directory: {error}"))?;
        crate::plugins::extract_zip(src, &unpacked)?;
        let skill_dir = find_archived_skill_dir(&unpacked)?;
        let skill_md = skill_dir.join("SKILL.md");
        (Some(temp_dir), skill_dir, skill_md)
    } else {
        return Err("select a skill folder, a SKILL.md file, or a ZIP archive".into());
    };

    let skill = superscience_skills::parse_skill_file(&skill_md)?;
    if skill.description.trim().is_empty() {
        return Err("SKILL.md is missing a description".into());
    }
    validate_skill_name(&skill.name)?;
    let dest = skills_dir.join(&skill.name);
    install_skill_dir(&skill_dir, &dest).map_err(|error| format!("install skill: {error}"))?;
    Ok(skill.name)
}

fn find_archived_skill_dir(root: &Path) -> Result<PathBuf, String> {
    if root.join("SKILL.md").is_file() {
        return Ok(root.to_path_buf());
    }
    let mut candidates = Vec::new();
    for entry in
        std::fs::read_dir(root).map_err(|error| format!("read skill ZIP contents: {error}"))?
    {
        let entry = entry.map_err(|error| format!("read skill ZIP entry: {error}"))?;
        if entry
            .file_type()
            .map_err(|error| format!("inspect skill ZIP entry: {error}"))?
            .is_dir()
            && entry.path().join("SKILL.md").is_file()
        {
            candidates.push(entry.path());
        }
    }
    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err("skill ZIP has no SKILL.md at its root or in a top-level folder".into()),
        _ => Err("skill ZIP contains more than one skill folder".into()),
    }
}

#[tauri::command]
pub(super) async fn remove_skill(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    name: String,
) -> Result<(), String> {
    validate_skill_name(&name)?;
    let dir = user_skills_dir()?.join(&name);
    if !dir.is_dir() {
        return Err("only user-added skills can be removed".into());
    }
    tokio::fs::remove_dir_all(&dir)
        .await
        .map_err(|e| format!("{e}"))?;
    let ap = state.active(window.label());
    if let Some(mut enabled) = load_enabled_skill_names(&state.store, &ap.id).await {
        enabled.remove(&name);
        let _ = save_enabled_skill_names(&state.store, &ap.id, &enabled).await;
    }
    let mut tags = load_skill_tags(&state.store).await;
    tags.remove(&name);
    let _ = save_skill_tags(&state.store, &tags).await;
    reload_host_skill_index(&state, window.label());
    clear_idle_agents(&state).await;
    Ok(())
}

const CATALOG_SKILL_MAX_BYTES: u64 = 256 * 1024 * 1024;
const CATALOG_SKILL_PROGRESS_EVENT: &str = "catalog-skill-install-progress";

#[derive(Clone, Copy)]
pub(super) struct CatalogSkillSpec {
    pub(super) id: &'static str,
    pub(super) owner: &'static str,
    pub(super) repo: &'static str,
    asset_prefix: &'static str,
}

const PPT_MASTER_SPEC: CatalogSkillSpec = CatalogSkillSpec {
    id: "ppt-master",
    owner: "hugohe3",
    repo: "ppt-master",
    asset_prefix: "ppt-master-skill-",
};

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    #[serde(default)]
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Clone, Serialize)]
struct CatalogSkillProgress {
    skill: String,
    received: u64,
    total: Option<u64>,
    phase: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CatalogSkillInstallResult {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pip_warning: Option<String>,
}

pub(super) fn catalog_skill_spec(id: &str) -> Result<&'static CatalogSkillSpec, String> {
    match id.trim() {
        "ppt-master" => Ok(&PPT_MASTER_SPEC),
        other => Err(format!("unknown catalog skill '{other}'")),
    }
}

fn select_catalog_skill_asset<'a>(
    spec: &CatalogSkillSpec,
    assets: &'a [GithubReleaseAsset],
) -> Result<&'a GithubReleaseAsset, String> {
    let matches: Vec<_> = assets
        .iter()
        .filter(|asset| {
            asset.name.starts_with(spec.asset_prefix)
                && asset.name.to_ascii_lowercase().ends_with(".zip")
        })
        .collect();
    match matches.as_slice() {
        [asset] => Ok(*asset),
        [] => Err(format!(
            "GitHub release for {}/{} has no {}*.zip skill package",
            spec.owner, spec.repo, spec.asset_prefix
        )),
        many => many
            .iter()
            .max_by_key(|asset| asset.name.as_str())
            .copied()
            .ok_or_else(|| "no catalog skill asset".into()),
    }
}

fn validate_catalog_download_url(spec: &CatalogSkillSpec, url: &str) -> Result<url::Url, String> {
    let parsed = url::Url::parse(url).map_err(|error| format!("invalid download URL: {error}"))?;
    if parsed.scheme() != "https" {
        return Err("catalog skill download must use HTTPS".into());
    }
    let host = parsed.host_str().unwrap_or_default();
    if host != "github.com" {
        return Err(format!(
            "catalog skill download host '{host}' is not allowed"
        ));
    }
    let expected = format!("/{}/{}/releases/download/", spec.owner, spec.repo);
    if !parsed.path().starts_with(&expected) {
        return Err(format!(
            "catalog skill download path must start with {expected}"
        ));
    }
    Ok(parsed)
}

fn emit_catalog_progress(
    app: &AppHandle,
    skill: &str,
    received: u64,
    total: Option<u64>,
    phase: &str,
) {
    let _ = app.emit(
        CATALOG_SKILL_PROGRESS_EVENT,
        CatalogSkillProgress {
            skill: skill.to_string(),
            received,
            total,
            phase: phase.to_string(),
        },
    );
}

pub(super) fn catalog_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("SuperScience-catalog-skill/1.3")
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(15 * 60))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.url().scheme() == "https" && attempt.previous().len() < 6 {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|error| format!("build catalog skill downloader: {error}"))
}

pub(super) async fn resolve_catalog_skill_zip(
    spec: &CatalogSkillSpec,
    client: &reqwest::Client,
) -> Result<(url::Url, Option<u64>), String> {
    let api = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        spec.owner, spec.repo
    );
    let release: GithubRelease = client
        .get(&api)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| format!("lookup catalog skill release: {error}"))?
        .error_for_status()
        .map_err(|error| format!("lookup catalog skill release: {error}"))?
        .json()
        .await
        .map_err(|error| format!("parse catalog skill release: {error}"))?;
    let asset = select_catalog_skill_asset(spec, &release.assets)?;
    let url = validate_catalog_download_url(spec, &asset.browser_download_url)?;
    let total = (asset.size > 0).then_some(asset.size);
    Ok((url, total))
}

pub(super) async fn download_catalog_skill_zip(
    app: &AppHandle,
    client: &reqwest::Client,
    skill: &str,
    url: url::Url,
    announced_total: Option<u64>,
    dest: &Path,
) -> Result<u64, String> {
    if announced_total.is_some_and(|length| length > CATALOG_SKILL_MAX_BYTES) {
        return Err("catalog skill download exceeds the archive size limit".into());
    }
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("download catalog skill: {error}"))?
        .error_for_status()
        .map_err(|error| format!("download catalog skill: {error}"))?;
    let total = response
        .content_length()
        .or(announced_total)
        .filter(|length| *length > 0);
    if total.is_some_and(|length| length > CATALOG_SKILL_MAX_BYTES) {
        return Err("catalog skill download exceeds the archive size limit".into());
    }
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(dest)
        .await
        .map_err(|error| format!("create catalog skill download: {error}"))?;
    let mut received = 0u64;
    let mut stream = response.bytes_stream();
    emit_catalog_progress(app, skill, 0, total, "download");
    let download_result = async {
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| format!("read catalog skill download: {error}"))?;
            received = received.saturating_add(chunk.len() as u64);
            if received > CATALOG_SKILL_MAX_BYTES {
                return Err("catalog skill download exceeds the archive size limit".to_string());
            }
            file.write_all(&chunk)
                .await
                .map_err(|error| format!("write catalog skill download: {error}"))?;
            emit_catalog_progress(app, skill, received, total, "download");
        }
        file.flush()
            .await
            .map_err(|error| format!("flush catalog skill download: {error}"))?;
        Ok::<_, String>(())
    }
    .await;
    if let Err(error) = download_result {
        drop(file);
        let _ = tokio::fs::remove_file(dest).await;
        return Err(error);
    }
    Ok(received)
}

fn catalog_python(app_data: &Path) -> Option<PathBuf> {
    let managed = superscience_runtime::PythonEnv::managed(app_data).python();
    if managed.is_file() {
        return Some(managed);
    }
    which::which("python3")
        .ok()
        .or_else(|| which::which("python").ok())
}

pub(super) fn install_catalog_skill_requirements(
    app_data: &Path,
    skill_dir: &Path,
) -> Result<(), String> {
    let requirements = skill_dir.join("requirements.txt");
    if !requirements.is_file() {
        return Ok(());
    }
    let python = catalog_python(app_data).ok_or_else(|| {
        "no Python interpreter found for skill dependencies (need 3.10+)".to_string()
    })?;
    let output = std::process::Command::new(&python)
        .args(["-m", "pip", "install", "--disable-pip-version-check", "-r"])
        .arg(&requirements)
        .output()
        .map_err(|error| format!("run pip: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = [stderr.trim(), stdout.trim()]
        .into_iter()
        .find(|text| !text.is_empty())
        .unwrap_or("pip install failed");
    Err(detail.chars().take(400).collect())
}

#[tauri::command]
pub(super) async fn install_catalog_skill(
    app: AppHandle,
    state: State<'_, AppState>,
    window: WebviewWindow,
    id: String,
) -> Result<CatalogSkillInstallResult, String> {
    let spec = catalog_skill_spec(&id)?;
    emit_catalog_progress(&app, spec.id, 0, None, "download");
    let downloads = state.app_data.join("catalog-skill-downloads");
    tokio::fs::create_dir_all(&downloads)
        .await
        .map_err(|error| format!("create catalog skill download directory: {error}"))?;
    let zip_path = downloads.join(format!("{}-{}.zip", spec.id, uuid::Uuid::new_v4()));
    let client = catalog_http_client()?;
    let (url, announced_total) = resolve_catalog_skill_zip(spec, &client).await?;
    let download_result =
        download_catalog_skill_zip(&app, &client, spec.id, url, announced_total, &zip_path).await;
    if let Err(error) = download_result {
        let _ = tokio::fs::remove_file(&zip_path).await;
        return Err(error);
    }
    emit_catalog_progress(&app, spec.id, 0, None, "install");
    let skills_dir = user_skills_dir()?;
    let zip_for_install = zip_path.clone();
    let skill_name =
        tokio::task::spawn_blocking(move || install_skill_source(&zip_for_install, &skills_dir))
            .await
            .map_err(|error| format!("{error}"))
            .and_then(|result| result);
    let _ = tokio::fs::remove_file(&zip_path).await;
    let skill_name = skill_name?;
    if skill_name != spec.id {
        return Err(format!(
            "installed skill '{skill_name}' does not match catalog id '{}'",
            spec.id
        ));
    }
    reload_host_skill_index(&state, window.label());
    let ap = state.active(window.label());
    if let Some(mut enabled) = load_enabled_skill_names(&state.store, &ap.id).await {
        enabled.insert(skill_name.clone());
        save_enabled_skill_names(&state.store, &ap.id, &enabled).await?;
    }
    clear_idle_agents(&state).await;
    emit_catalog_progress(&app, spec.id, 0, None, "pip");
    let skill_dir = user_skills_dir()?.join(&skill_name);
    let app_data = state.app_data.clone();
    let pip_warning = tokio::task::spawn_blocking(move || {
        install_catalog_skill_requirements(&app_data, &skill_dir).err()
    })
    .await
    .ok()
    .flatten();
    Ok(CatalogSkillInstallResult {
        name: skill_name,
        pip_warning,
    })
}

pub(super) fn reload_host_skill_index(state: &AppState, label: &str) {
    let mut ap = state.active(label);
    ap.skills = Arc::new(load_skill_index(&ap.root));
    state.set_active(label, ap);
}

pub(super) fn copy_dir_recursive(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

/// Install a skill directory, replacing an existing user-installed copy.
///
/// The new tree is copied to a sibling staging directory first. Only after the
/// copy succeeds do we move the old tree aside and atomically put the staged
/// tree in its place. This prevents a failed update from leaving a partially
/// copied skill behind.
pub(super) fn install_skill_dir(from: &Path, to: &Path) -> Result<bool, String> {
    let parent = to
        .parent()
        .ok_or_else(|| "skill destination has no parent directory".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;

    if to.exists() {
        if !to.is_dir() {
            return Err(format!(
                "skill destination '{}' is not a directory",
                to.display()
            ));
        }
        if same_file::is_same_file(from, to).unwrap_or(false) {
            return Ok(true);
        }
    }

    let name = to
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("skill");
    let token = uuid::Uuid::new_v4();
    let staging = parent.join(format!(".{name}-install-{token}"));
    let backup = parent.join(format!(".{name}-backup-{token}"));

    if let Err(error) = copy_dir_recursive(from, &staging) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error.to_string());
    }

    let replaced = to.exists();
    if replaced {
        if let Err(error) = std::fs::rename(to, &backup) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(error.to_string());
        }
    }

    if let Err(error) = std::fs::rename(&staging, to) {
        let restore_error = if replaced {
            std::fs::rename(&backup, to).err()
        } else {
            None
        };
        let _ = std::fs::remove_dir_all(&staging);
        return match restore_error {
            Some(restore_error) => Err(format!(
                "{error}; restoring the previous skill also failed: {restore_error}"
            )),
            None => Err(error.to_string()),
        };
    }

    if replaced {
        if let Err(error) = std::fs::remove_dir_all(&backup) {
            tracing::warn!(
                path = %backup.display(),
                %error,
                "could not remove replaced skill backup"
            );
        }
    }
    Ok(replaced)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("superscience-skill-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).expect("create test directory");
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn installing_same_named_skill_replaces_the_existing_tree() {
        let temp = TestDir::new();
        let source = temp.0.join("source");
        let destination = temp.0.join("installed").join("example");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(source.join("SKILL.md"), "new instructions").unwrap();
        std::fs::write(source.join("new.txt"), "new resource").unwrap();
        std::fs::write(destination.join("SKILL.md"), "old instructions").unwrap();
        std::fs::write(destination.join("stale.txt"), "stale resource").unwrap();

        assert_eq!(install_skill_dir(&source, &destination), Ok(true));
        assert_eq!(
            std::fs::read_to_string(destination.join("SKILL.md")).unwrap(),
            "new instructions"
        );
        assert!(destination.join("new.txt").is_file());
        assert!(!destination.join("stale.txt").exists());
    }

    #[test]
    fn failed_skill_copy_keeps_the_existing_tree() {
        let temp = TestDir::new();
        let source = temp.0.join("missing-source");
        let destination = temp.0.join("installed").join("example");
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(destination.join("SKILL.md"), "old instructions").unwrap();

        assert!(install_skill_dir(&source, &destination).is_err());
        assert_eq!(
            std::fs::read_to_string(destination.join("SKILL.md")).unwrap(),
            "old instructions"
        );
    }

    #[test]
    fn zip_skill_import_installs_a_single_top_level_skill_folder() {
        let temp = TestDir::new();
        let archive_path = temp.0.join("ggtree-visualization.zip");
        let file = std::fs::File::create(&archive_path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file(
                "ggtree-visualization/SKILL.md",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive
            .write_all(b"---\nname: ggtree-visualization\ndescription: Draw trees\n---\n# Skill")
            .unwrap();
        archive
            .start_file(
                "ggtree-visualization/assets/template.R",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive.write_all(b"plot(tree)").unwrap();
        archive.finish().unwrap();

        let installed = temp.0.join("installed");
        assert_eq!(
            install_skill_source(&archive_path, &installed),
            Ok("ggtree-visualization".into())
        );
        assert!(installed.join("ggtree-visualization/SKILL.md").is_file());
        assert_eq!(
            std::fs::read_to_string(installed.join("ggtree-visualization/assets/template.R"))
                .unwrap(),
            "plot(tree)"
        );
    }

    #[test]
    fn zip_skill_import_rejects_multiple_skill_folders() {
        let temp = TestDir::new();
        let archive_path = temp.0.join("skills.zip");
        let file = std::fs::File::create(&archive_path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        for name in ["one", "two"] {
            archive
                .start_file(
                    format!("{name}/SKILL.md"),
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            archive
                .write_all(format!("---\nname: {name}\ndescription: Test\n---\n").as_bytes())
                .unwrap();
        }
        archive.finish().unwrap();

        let error = install_skill_source(&archive_path, &temp.0.join("installed")).unwrap_err();
        assert_eq!(error, "skill ZIP contains more than one skill folder");
    }

    #[test]
    fn catalog_skill_spec_allows_only_ppt_master() {
        assert_eq!(catalog_skill_spec("ppt-master").unwrap().id, "ppt-master");
        assert_eq!(
            catalog_skill_spec(" ppt-master ").unwrap().repo,
            "ppt-master"
        );
        assert!(catalog_skill_spec("nature-paper2ppt").is_err());
        assert!(catalog_skill_spec("../ppt-master").is_err());
        assert!(catalog_skill_spec("").is_err());
    }

    #[test]
    fn catalog_skill_asset_picks_skill_only_zip() {
        let assets = vec![
            GithubReleaseAsset {
                name: "Source code (zip)".into(),
                browser_download_url: "https://github.com/hugohe3/ppt-master/archive/refs/tags/v4.7.0.zip".into(),
                size: 1,
            },
            GithubReleaseAsset {
                name: "ppt-master-skill-v4.7.0.zip".into(),
                browser_download_url: "https://github.com/hugohe3/ppt-master/releases/download/v4.7.0/ppt-master-skill-v4.7.0.zip".into(),
                size: 58_747_948,
            },
        ];
        let asset = select_catalog_skill_asset(&PPT_MASTER_SPEC, &assets).unwrap();
        assert_eq!(asset.name, "ppt-master-skill-v4.7.0.zip");
        assert!(
            validate_catalog_download_url(&PPT_MASTER_SPEC, &asset.browser_download_url).is_ok()
        );
    }

    #[test]
    fn catalog_skill_asset_rejects_repo_zipball_and_foreign_hosts() {
        let assets = vec![GithubReleaseAsset {
            name: "ppt-master-v4.7.0.zip".into(),
            browser_download_url:
                "https://github.com/hugohe3/ppt-master/archive/refs/tags/v4.7.0.zip".into(),
            size: 10,
        }];
        assert!(select_catalog_skill_asset(&PPT_MASTER_SPEC, &assets).is_err());
        assert!(validate_catalog_download_url(
            &PPT_MASTER_SPEC,
            "https://evil.example/ppt-master-skill-v4.7.0.zip"
        )
        .is_err());
        assert!(validate_catalog_download_url(
            &PPT_MASTER_SPEC,
            "https://github.com/other/ppt-master/releases/download/v4.7.0/ppt-master-skill-v4.7.0.zip"
        )
        .is_err());
        assert!(validate_catalog_download_url(
            &PPT_MASTER_SPEC,
            "http://github.com/hugohe3/ppt-master/releases/download/v4.7.0/ppt-master-skill-v4.7.0.zip"
        )
        .is_err());
    }

    #[test]
    fn catalog_skill_zip_fixture_installs_named_skill() {
        let temp = TestDir::new();
        let archive_path = temp.0.join("ppt-master-skill-v4.7.0.zip");
        let file = std::fs::File::create(&archive_path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file(
                "ppt-master/SKILL.md",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive
            .write_all(b"---\nname: ppt-master\ndescription: Native editable PPT\n---\n# Skill")
            .unwrap();
        archive
            .start_file(
                "ppt-master/requirements.txt",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive.write_all(b"lxml\n").unwrap();
        archive.finish().unwrap();

        let installed = temp.0.join("installed");
        assert_eq!(
            install_skill_source(&archive_path, &installed),
            Ok("ppt-master".into())
        );
        assert!(installed.join("ppt-master/SKILL.md").is_file());
        assert_eq!(
            std::fs::read_to_string(installed.join("ppt-master/requirements.txt")).unwrap(),
            "lxml\n"
        );
    }

    #[test]
    fn reload_enables_new_skills_without_reviving_disabled_existing_skills() {
        let previous = HashSet::from(["enabled".into(), "disabled".into()]);
        let enabled = Some(HashSet::from(["enabled".into()]));

        let updated = enabled_names_after_reload(
            enabled,
            &previous,
            ["enabled", "disabled", "new-project-skill"],
        )
        .unwrap();

        assert!(updated.contains("enabled"));
        assert!(!updated.contains("disabled"));
        assert!(updated.contains("new-project-skill"));
    }
}
