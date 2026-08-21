//! Auto-update for allowlisted, GitHub-sourced skill packs.
//!
//! Updates install into `~/.superscience/skills` overlays; the signed bundled
//! catalog is never rewritten. Network failures are collected into the report
//! instead of failing the command.

use super::{
    clear_idle_agents, load_enabled_skill_names, save_enabled_skill_names, skill_commands, AppState,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use superscience_skills::update::{
    apply_pack_local_patches, archive_inner_root, collect_declared_skill_dirs,
    needs_remote_install, normalize_academic_shared_layout, overlay_stale_vs_bundled,
    pack_overlay_skill_names, PinSource, VendorPackSpec, VENDOR_PACKS,
};
use tauri::{AppHandle, State, WebviewWindow};

const SKILL_UPDATE_ENABLED_KEY: &str = "skill_update_enabled";
const SKILL_UPDATE_REPORT_KEY: &str = "skill_update_last_report";
const ARCHIVE_MAX_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct SkillUpdateReport {
    pub enabled: bool,
    pub checked: bool,
    #[serde(default)]
    pub last_check_at: Option<String>,
    #[serde(default)]
    pub last_check_at_ms: Option<i64>,
    #[serde(default)]
    pub updated: Vec<String>,
    #[serde(default)]
    pub skipped: Vec<String>,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub dropped_overlays: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SkillUpdateCandidate {
    pub id: String,
    #[serde(default)]
    pub current_pin: String,
    #[serde(default)]
    pub remote_pin: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SkillUpdatePreview {
    #[serde(default)]
    pub available: Vec<SkillUpdateCandidate>,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PackOverlayState {
    pin: String,
    #[serde(default)]
    source_uri: String,
    #[serde(default)]
    archive_sha256: String,
    #[serde(default)]
    installed_by: String,
}

#[derive(Debug, Deserialize)]
struct GithubCommit {
    sha: String,
}

#[derive(Debug, Deserialize)]
struct GithubTag {
    name: String,
}

fn vendor_state_path(skills_dir: &Path, pack_id: &str) -> PathBuf {
    skills_dir.join(".vendor").join(format!("{pack_id}.json"))
}

fn read_pack_state(skills_dir: &Path, pack_id: &str) -> Option<PackOverlayState> {
    let path = vendor_state_path(skills_dir, pack_id);
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_pack_state(
    skills_dir: &Path,
    pack_id: &str,
    state: &PackOverlayState,
) -> Result<(), String> {
    let path = vendor_state_path(skills_dir, pack_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(state).map_err(|error| error.to_string())?;
    std::fs::write(&path, text).map_err(|error| error.to_string())
}

fn remove_pack_state(skills_dir: &Path, pack_id: &str) {
    let _ = std::fs::remove_file(vendor_state_path(skills_dir, pack_id));
}

async fn load_skill_update_enabled(store: &superscience_store::Store) -> bool {
    store
        .get_setting(SKILL_UPDATE_ENABLED_KEY)
        .await
        .ok()
        .flatten()
        .and_then(|value| serde_json::from_str::<bool>(&value).ok())
        .unwrap_or(true)
}

async fn save_skill_update_enabled(
    store: &superscience_store::Store,
    enabled: bool,
) -> Result<(), String> {
    store
        .set_setting(SKILL_UPDATE_ENABLED_KEY, &enabled.to_string())
        .await
        .map_err(|error| error.to_string())
}

async fn load_last_report(store: &superscience_store::Store) -> SkillUpdateReport {
    store
        .get_setting(SKILL_UPDATE_REPORT_KEY)
        .await
        .ok()
        .flatten()
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default()
}

async fn save_last_report(
    store: &superscience_store::Store,
    report: &SkillUpdateReport,
) -> Result<(), String> {
    let text = serde_json::to_string(report).map_err(|error| error.to_string())?;
    store
        .set_setting(SKILL_UPDATE_REPORT_KEY, &text)
        .await
        .map_err(|error| error.to_string())
}

fn validate_git_ref(value: &str) -> Result<(), String> {
    if value.is_empty() || value.contains("..") {
        return Err("invalid git ref".into());
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/'))
    {
        return Err(format!("invalid git ref '{value}'"));
    }
    Ok(())
}

pub(super) fn archive_zip_url(owner: &str, repo: &str, git_ref: &str) -> Result<String, String> {
    validate_git_ref(git_ref)?;
    Ok(format!(
        "https://github.com/{owner}/{repo}/archive/{git_ref}.zip"
    ))
}

pub(super) fn validate_github_archive_url(
    owner: &str,
    repo: &str,
    url: &str,
) -> Result<url::Url, String> {
    let parsed = url::Url::parse(url).map_err(|error| format!("invalid archive URL: {error}"))?;
    if parsed.scheme() != "https" {
        return Err("skill archive download must use HTTPS".into());
    }
    let host = parsed.host_str().unwrap_or_default();
    if host != "github.com" {
        return Err(format!("skill archive host '{host}' is not allowed"));
    }
    let expected = format!("/{owner}/{repo}/archive/");
    if !parsed.path().starts_with(&expected) {
        return Err(format!("skill archive path must start with {expected}"));
    }
    if parsed.path().contains("..") {
        return Err("skill archive path must not contain '..'".into());
    }
    Ok(parsed)
}

fn archive_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("SuperScience-skill-update/1.5")
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(15 * 60))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let host = attempt.url().host_str().unwrap_or_default();
            let allowed = attempt.url().scheme() == "https"
                && matches!(
                    host,
                    "github.com" | "codeload.github.com" | "api.github.com"
                )
                && attempt.previous().len() < 6;
            if allowed {
                attempt.follow()
            } else {
                attempt.error("disallowed skill archive redirect")
            }
        }))
        .build()
        .map_err(|error| format!("build skill updater: {error}"))
}

async fn fetch_default_branch_sha(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
) -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/commits?per_page=1");
    let commits: Vec<GithubCommit> = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| format!("lookup {owner}/{repo} commits: {error}"))?
        .error_for_status()
        .map_err(|error| format!("lookup {owner}/{repo} commits: {error}"))?
        .json()
        .await
        .map_err(|error| format!("parse {owner}/{repo} commits: {error}"))?;
    commits
        .into_iter()
        .next()
        .map(|commit| commit.sha)
        .ok_or_else(|| format!("{owner}/{repo} returned no commits"))
}

async fn fetch_latest_semver_tag(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
) -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/tags?per_page=100");
    let tags: Vec<GithubTag> = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| format!("lookup {owner}/{repo} tags: {error}"))?
        .error_for_status()
        .map_err(|error| format!("lookup {owner}/{repo} tags: {error}"))?
        .json()
        .await
        .map_err(|error| format!("parse {owner}/{repo} tags: {error}"))?;
    tags.into_iter()
        .filter_map(|tag| {
            superscience_skills::update::parse_semver_pin(&tag.name)
                .map(|version| (version, tag.name))
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, name)| name)
        .ok_or_else(|| format!("{owner}/{repo} has no semver tags"))
}

async fn fetch_remote_pin(
    client: &reqwest::Client,
    pack: &VendorPackSpec,
) -> Result<(String, String), String> {
    match pack.pin_source {
        PinSource::DefaultBranch => {
            let sha = fetch_default_branch_sha(client, pack.owner, pack.repo).await?;
            Ok((sha.clone(), sha))
        }
        PinSource::LatestSemverTag => {
            let tag = fetch_latest_semver_tag(client, pack.owner, pack.repo).await?;
            Ok((tag.clone(), tag))
        }
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let metadata =
        std::fs::metadata(path).map_err(|error| format!("stat skill archive: {error}"))?;
    if metadata.len() > ARCHIVE_MAX_BYTES {
        return Err("skill archive exceeds the size limit".into());
    }
    let mut file =
        std::fs::File::open(path).map_err(|error| format!("open skill archive: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("read skill archive: {error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

async fn download_archive(
    client: &reqwest::Client,
    url: url::Url,
    dest: &Path,
) -> Result<(), String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("download skill archive: {error}"))?
        .error_for_status()
        .map_err(|error| format!("download skill archive: {error}"))?;
    let total = response.content_length();
    if total.is_some_and(|length| length > ARCHIVE_MAX_BYTES) {
        return Err("skill archive exceeds the size limit".into());
    }
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(dest)
        .await
        .map_err(|error| format!("create skill archive: {error}"))?;
    let mut received = 0u64;
    let mut stream = response.bytes_stream();
    let download_result = async {
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| format!("read skill archive: {error}"))?;
            received = received.saturating_add(chunk.len() as u64);
            if received > ARCHIVE_MAX_BYTES {
                return Err("skill archive exceeds the size limit".to_string());
            }
            file.write_all(&chunk)
                .await
                .map_err(|error| format!("write skill archive: {error}"))?;
        }
        file.flush()
            .await
            .map_err(|error| format!("flush skill archive: {error}"))?;
        Ok::<_, String>(())
    }
    .await;
    if let Err(error) = download_result {
        drop(file);
        let _ = tokio::fs::remove_file(dest).await;
        return Err(error);
    }
    Ok(())
}

fn drop_stale_overlays(skills_dir: &Path) -> Vec<String> {
    let mut dropped = Vec::new();
    for pack in VENDOR_PACKS {
        let Some(state) = read_pack_state(skills_dir, pack.id) else {
            continue;
        };
        if state.installed_by != "skill-auto-update" {
            continue;
        }
        if !overlay_stale_vs_bundled(&state.pin, pack.bundled_pin, pack.pin_kind) {
            continue;
        }
        for skill in pack_overlay_skill_names(pack) {
            let dest = skills_dir.join(skill);
            if dest.is_dir() {
                let _ = std::fs::remove_dir_all(&dest);
            }
        }
        remove_pack_state(skills_dir, pack.id);
        dropped.push(pack.id.to_string());
    }
    dropped
}

fn install_pack_from_archive(
    archive: &Path,
    skills_dir: &Path,
    pack: &VendorPackSpec,
    remote_pin: &str,
    source_uri: &str,
) -> Result<Vec<String>, String> {
    let unpack_root = skills_dir.join(format!(".vendor-unpack-{}", uuid::Uuid::new_v4()));
    let _cleanup = ScopeRemoveDir(unpack_root.clone());
    std::fs::create_dir_all(&unpack_root).map_err(|error| error.to_string())?;
    crate::plugins::extract_zip(archive, &unpack_root)?;
    let inner = archive_inner_root(&unpack_root)?;
    let selected = collect_declared_skill_dirs(&inner, pack.skills)?;
    let mut installed = Vec::new();
    for (name, source) in selected.dirs {
        let dest = skills_dir.join(&name);
        skill_commands::install_skill_dir(&source, &dest)?;
        apply_pack_local_patches(&name, &dest)?;
        installed.push(name);
    }
    if pack.id == "academic-research-skills" && normalize_academic_shared_layout(skills_dir)? {
        installed.push(superscience_skills::update::ACADEMIC_SHARED_NAME.to_string());
    }
    let archive_sha256 = sha256_file(archive).unwrap_or_default();
    write_pack_state(
        skills_dir,
        pack.id,
        &PackOverlayState {
            pin: remote_pin.to_string(),
            source_uri: source_uri.to_string(),
            archive_sha256,
            installed_by: "skill-auto-update".into(),
        },
    )?;
    Ok(installed)
}

struct ScopeRemoveDir(PathBuf);

impl Drop for ScopeRemoveDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn release_tag_from_download_url(url: &url::Url) -> Option<String> {
    let segs: Vec<_> = url.path_segments()?.collect();
    (segs.len() >= 6 && segs[2] == "releases" && segs[3] == "download").then(|| segs[4].to_string())
}

async fn update_ppt_master_if_needed(
    app: &AppHandle,
    skills_dir: &Path,
    app_data: &Path,
) -> Result<Option<String>, String> {
    let skill_dir = skills_dir.join("ppt-master");
    if !skill_dir.join("SKILL.md").is_file() {
        return Ok(None);
    }
    let overlay_pin = read_pack_state(skills_dir, "ppt-master").map(|state| state.pin);
    let client = skill_commands::catalog_http_client()?;
    let spec = skill_commands::catalog_skill_spec("ppt-master")?;
    let (url, announced_total) = skill_commands::resolve_catalog_skill_zip(spec, &client).await?;
    let remote_pin = release_tag_from_download_url(&url)
        .ok_or_else(|| "ppt-master release URL is missing a tag".to_string())?;
    if !needs_remote_install(overlay_pin.as_deref(), "", &remote_pin) {
        return Ok(None);
    }
    let downloads = skills_dir.join(".vendor-downloads");
    tokio::fs::create_dir_all(&downloads)
        .await
        .map_err(|error| error.to_string())?;
    let zip_path = downloads.join(format!("ppt-master-{}.zip", uuid::Uuid::new_v4()));
    let download_result = skill_commands::download_catalog_skill_zip(
        app,
        &client,
        "ppt-master",
        url.clone(),
        announced_total,
        &zip_path,
    )
    .await;
    if let Err(error) = download_result {
        let _ = tokio::fs::remove_file(&zip_path).await;
        return Err(error);
    }
    let zip_for_install = zip_path.clone();
    let skills_dir_for_install = skills_dir.to_path_buf();
    let install_result = tokio::task::spawn_blocking(move || {
        skill_commands::install_skill_source(&zip_for_install, &skills_dir_for_install)
    })
    .await
    .map_err(|error| format!("{error}"))
    .and_then(|result| result);
    let _ = tokio::fs::remove_file(&zip_path).await;
    let name = install_result?;
    if name != "ppt-master" {
        return Err(format!(
            "installed skill '{name}' does not match ppt-master"
        ));
    }
    write_pack_state(
        skills_dir,
        "ppt-master",
        &PackOverlayState {
            pin: remote_pin.clone(),
            source_uri: url.to_string(),
            archive_sha256: String::new(),
            installed_by: "skill-auto-update".into(),
        },
    )?;
    let pip_dir = skills_dir.join("ppt-master");
    let app_data = app_data.to_path_buf();
    let _ = tokio::task::spawn_blocking(move || {
        skill_commands::install_catalog_skill_requirements(&app_data, &pip_dir)
    })
    .await;
    Ok(Some("ppt-master".into()))
}

async fn enable_skill_names(state: &AppState, label: &str, names: &[String]) -> Result<(), String> {
    if names.is_empty() {
        return Ok(());
    }
    let ap = state.active(label);
    if let Some(mut enabled) = load_enabled_skill_names(&state.store, &ap.id).await {
        for name in names {
            enabled.insert(name.clone());
        }
        save_enabled_skill_names(&state.store, &ap.id, &enabled).await?;
        clear_idle_agents(state).await;
    }
    Ok(())
}

async fn preview_ppt_master(skills_dir: &Path) -> Result<Option<SkillUpdateCandidate>, String> {
    let skill_dir = skills_dir.join("ppt-master");
    if !skill_dir.join("SKILL.md").is_file() {
        return Ok(None);
    }
    let overlay_pin = read_pack_state(skills_dir, "ppt-master").map(|state| state.pin);
    let client = skill_commands::catalog_http_client()?;
    let spec = skill_commands::catalog_skill_spec("ppt-master")?;
    let (url, _) = skill_commands::resolve_catalog_skill_zip(spec, &client).await?;
    let remote_pin = release_tag_from_download_url(&url)
        .ok_or_else(|| "ppt-master release URL is missing a tag".to_string())?;
    if !needs_remote_install(overlay_pin.as_deref(), "", &remote_pin) {
        return Ok(None);
    }
    Ok(Some(SkillUpdateCandidate {
        id: "ppt-master".into(),
        current_pin: overlay_pin.unwrap_or_default(),
        remote_pin,
    }))
}

async fn preview_vendor_packs(skills_dir: &Path) -> SkillUpdatePreview {
    let mut preview = SkillUpdatePreview::default();
    let client = match archive_http_client() {
        Ok(client) => client,
        Err(error) => {
            preview.errors.push(error);
            return preview;
        }
    };
    for pack in VENDOR_PACKS {
        let overlay_pin = read_pack_state(skills_dir, pack.id).map(|state| state.pin);
        match fetch_remote_pin(&client, pack).await {
            Ok((remote_pin, _)) => {
                if needs_remote_install(overlay_pin.as_deref(), pack.bundled_pin, &remote_pin) {
                    preview.available.push(SkillUpdateCandidate {
                        id: pack.id.to_string(),
                        current_pin: overlay_pin.unwrap_or_else(|| pack.bundled_pin.to_string()),
                        remote_pin,
                    });
                }
            }
            Err(error) => preview.errors.push(format!("{}: {error}", pack.id)),
        }
    }
    match preview_ppt_master(skills_dir).await {
        Ok(Some(item)) => preview.available.push(item),
        Ok(None) => {}
        Err(error) => preview.errors.push(format!("ppt-master: {error}")),
    }
    preview
}

async fn run_skill_update_check(
    app: &AppHandle,
    state: &AppState,
    label: &str,
    force: bool,
) -> Result<SkillUpdateReport, String> {
    let enabled = load_skill_update_enabled(&state.store).await;
    let skills_dir = skill_commands::user_skills_dir()?;
    std::fs::create_dir_all(&skills_dir).map_err(|error| error.to_string())?;

    let mut report = SkillUpdateReport {
        enabled,
        checked: false,
        ..SkillUpdateReport::default()
    };
    report.dropped_overlays = drop_stale_overlays(&skills_dir);

    if !enabled && !force {
        if !report.dropped_overlays.is_empty() {
            skill_commands::reload_host_skill_index(state, label);
        }
        save_last_report(&state.store, &report).await?;
        return Ok(report);
    }

    let now = chrono::Local::now();
    report.checked = true;
    report.last_check_at = Some(now.to_rfc3339());
    report.last_check_at_ms = Some(now.timestamp_millis());

    let client = match archive_http_client() {
        Ok(client) => client,
        Err(error) => {
            report.errors.push(error);
            save_last_report(&state.store, &report).await?;
            return Ok(report);
        }
    };

    let mut newly_enabled = Vec::new();
    for pack in VENDOR_PACKS {
        match update_one_pack(&client, &skills_dir, pack).await {
            Ok(PackOutcome::Updated(names)) => {
                newly_enabled.extend(names);
                report.updated.push(pack.id.to_string());
            }
            Ok(PackOutcome::Skipped) => report.skipped.push(pack.id.to_string()),
            Err(error) => {
                tracing::warn!(target: "wisp", pack = pack.id, %error, "skill auto-update failed");
                report.errors.push(format!("{}: {error}", pack.id));
            }
        }
    }

    match update_ppt_master_if_needed(app, &skills_dir, &state.app_data).await {
        Ok(Some(_)) => {
            newly_enabled.push("ppt-master".into());
            report.updated.push("ppt-master".into());
        }
        Ok(None) => report.skipped.push("ppt-master".into()),
        Err(error) => {
            tracing::warn!(target: "wisp", %error, "ppt-master auto-update failed");
            report.errors.push(format!("ppt-master: {error}"));
        }
    }

    if !report.updated.is_empty() || !report.dropped_overlays.is_empty() {
        skill_commands::reload_host_skill_index(state, label);
        let _ = enable_skill_names(state, label, &newly_enabled).await;
    }

    save_last_report(&state.store, &report).await?;
    Ok(report)
}

enum PackOutcome {
    Updated(Vec<String>),
    Skipped,
}

async fn update_one_pack(
    client: &reqwest::Client,
    skills_dir: &Path,
    pack: &VendorPackSpec,
) -> Result<PackOutcome, String> {
    let overlay_pin = read_pack_state(skills_dir, pack.id).map(|state| state.pin);
    let (remote_pin, archive_ref) = fetch_remote_pin(client, pack).await?;
    if !needs_remote_install(overlay_pin.as_deref(), pack.bundled_pin, &remote_pin) {
        return Ok(PackOutcome::Skipped);
    }
    let source_uri = archive_zip_url(pack.owner, pack.repo, &archive_ref)?;
    let url = validate_github_archive_url(pack.owner, pack.repo, &source_uri)?;
    let downloads = skills_dir.join(".vendor-downloads");
    tokio::fs::create_dir_all(&downloads)
        .await
        .map_err(|error| error.to_string())?;
    let zip_path = downloads.join(format!("{}-{}.zip", pack.id, uuid::Uuid::new_v4()));
    let download_result = download_archive(client, url, &zip_path).await;
    if let Err(error) = download_result {
        let _ = tokio::fs::remove_file(&zip_path).await;
        return Err(error);
    }
    let zip_for_install = zip_path.clone();
    let skills_dir_for_install = skills_dir.to_path_buf();
    let pack_id = pack.id;
    let remote_pin_owned = remote_pin.clone();
    let source_uri_owned = source_uri.clone();
    let pack_copy = *pack;
    let install_result = tokio::task::spawn_blocking(move || {
        install_pack_from_archive(
            &zip_for_install,
            &skills_dir_for_install,
            &pack_copy,
            &remote_pin_owned,
            &source_uri_owned,
        )
    })
    .await
    .map_err(|error| format!("{error}"))
    .and_then(|result| result);
    let _ = tokio::fs::remove_file(&zip_path).await;
    let names = install_result?;
    let _ = pack_id;
    Ok(PackOutcome::Updated(names))
}

#[tauri::command]
pub(super) async fn get_skill_update_enabled(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(load_skill_update_enabled(&state.store).await)
}

#[tauri::command]
pub(super) async fn set_skill_update_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    save_skill_update_enabled(&state.store, enabled).await?;
    Ok(enabled)
}

#[tauri::command]
pub(super) async fn get_skill_update_status(
    state: State<'_, AppState>,
) -> Result<SkillUpdateReport, String> {
    let mut report = load_last_report(&state.store).await;
    report.enabled = load_skill_update_enabled(&state.store).await;
    Ok(report)
}

#[tauri::command]
pub(super) async fn preview_skill_updates() -> Result<SkillUpdatePreview, String> {
    let skills_dir = skill_commands::user_skills_dir()?;
    Ok(preview_vendor_packs(&skills_dir).await)
}

#[tauri::command]
pub(super) async fn check_skill_updates(
    app: AppHandle,
    state: State<'_, AppState>,
    window: WebviewWindow,
    force: Option<bool>,
) -> Result<SkillUpdateReport, String> {
    run_skill_update_check(&app, &state, window.label(), force.unwrap_or(false)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_url_rejects_path_traversal() {
        assert!(archive_zip_url("a", "b", "../evil").is_err());
        assert!(validate_github_archive_url(
            "Yuan1z0825",
            "nature-skills",
            "https://evil.example/Yuan1z0825/nature-skills/archive/sha.zip"
        )
        .is_err());
        assert!(validate_github_archive_url(
            "Yuan1z0825",
            "nature-skills",
            "https://github.com/Yuan1z0825/nature-skills/releases/download/v1/x.zip"
        )
        .is_err());
        let ok = validate_github_archive_url(
            "Yuan1z0825",
            "nature-skills",
            "https://github.com/Yuan1z0825/nature-skills/archive/c171989db699bd601d4373912b3fb8db96ecc95b.zip",
        );
        assert!(ok.is_ok());
    }

    #[test]
    fn release_tag_parses_from_download_url() {
        let url = url::Url::parse(
            "https://github.com/hugohe3/ppt-master/releases/download/v4.7.0/ppt-master-skill-v4.7.0.zip",
        )
        .unwrap();
        assert_eq!(
            release_tag_from_download_url(&url).as_deref(),
            Some("v4.7.0")
        );
    }

    #[test]
    fn skill_source_paths_put_overlays_before_bundled() {
        let project = PathBuf::from("/proj");
        let home = PathBuf::from("/home/me");
        let bundled = PathBuf::from("/app/skills");
        let paths = superscience_skills::skill_source_paths(
            &project,
            Some(&home),
            [PathBuf::from("/extra")],
            Some(bundled.clone()),
        );
        assert_eq!(
            paths.first().unwrap().0,
            project.join(".superscience/skills")
        );
        assert_eq!(paths.last().unwrap().0, bundled);
        assert!(paths
            .iter()
            .any(|(path, _)| path == &home.join(".superscience/skills")));
    }
}
