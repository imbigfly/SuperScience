//! Optional, user-confirmed application updates via the TCTOKEN client API.
//!
//! Check: `GET /api/client/update/check`. Download the returned `download_url`,
//! verify SHA-256 when provided, then open the package with the system installer.

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Mutex as StdMutex;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tokio::io::AsyncWriteExt;

const CHECK_PATH: &str = "/api/client/update/check";

#[derive(Clone, Serialize)]
pub(super) struct UpdateCheck {
    pub(super) current_version: String,
    pub(super) latest_version: String,
    pub(super) update_available: bool,
    pub(super) release_url: String,
    pub(super) notes: String,
    pub(super) install_supported: bool,
    pub(super) downloaded: bool,
    pub(super) downloading: bool,
    pub(super) force_update: bool,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    success: bool,
    #[serde(default)]
    message: String,
    data: Option<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ClientUpdateData {
    /// Absent on the current schema; `false` still means the server declined an update.
    #[serde(default)]
    pub(super) has_update: Option<bool>,
    #[serde(default)]
    pub(super) force_update: bool,
    #[serde(default)]
    pub(super) current_version: String,
    #[serde(default)]
    pub(super) latest_version: String,
    #[serde(default)]
    pub(super) package_name: String,
    #[serde(default)]
    pub(super) download_url: String,
    #[serde(default)]
    pub(super) release_notes: String,
    #[serde(default)]
    pub(super) checksum: String,
    #[serde(default)]
    pub(super) file_size: i64,
    /// Resolved storage key from the server, e.g. `macos-m`.
    #[serde(default)]
    pub(super) platform: String,
}

struct PendingUpdate {
    current_version: String,
    latest_version: String,
    download_url: String,
    checksum: String,
    package_name: String,
    notes: String,
    force_update: bool,
    local_path: Option<PathBuf>,
    downloading: bool,
}

#[derive(Default)]
pub(super) struct PendingAppUpdate(StdMutex<Option<PendingUpdate>>);

impl PendingUpdate {
    fn check(&self) -> UpdateCheck {
        UpdateCheck {
            current_version: self.current_version.clone(),
            latest_version: self.latest_version.clone(),
            update_available: true,
            release_url: self.download_url.clone(),
            notes: self.notes.clone(),
            install_supported: !self.download_url.is_empty(),
            downloaded: self.local_path.is_some(),
            downloading: self.downloading,
            force_update: self.force_update,
        }
    }
}

pub(super) fn client_update_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

pub(super) fn client_update_arch() -> &'static str {
    client_update_arch_for(client_update_platform(), {
        if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            "x86_64"
        }
    })
}

pub(super) fn client_update_arch_for(os: &str, arch: &str) -> &'static str {
    match (os, arch) {
        ("macos", "aarch64") => "m",
        ("macos", _) => "intel",
        (_, "aarch64") => "arm64",
        _ => "x64",
    }
}

pub(super) fn update_check_from_api(
    running_version: &str,
    data: ClientUpdateData,
) -> Result<UpdateCheck, String> {
    let current_text = if data.current_version.trim().is_empty() {
        running_version
    } else {
        data.current_version.trim()
    };
    let latest_text = data.latest_version.trim().trim_start_matches(['v', 'V']);
    let current = parse_semver(current_text, "current")?;
    let latest = if latest_text.is_empty() {
        current.clone()
    } else {
        parse_semver(latest_text, "latest")?
    };
    let newer = latest > current;
    let update_available = match data.has_update {
        Some(false) => false,
        Some(true) | None => newer,
    };
    let download_url = data.download_url.trim().to_string();

    Ok(UpdateCheck {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        update_available,
        release_url: download_url.clone(),
        notes: data.release_notes,
        install_supported: update_available && !download_url.is_empty(),
        downloaded: false,
        downloading: false,
        force_update: update_available && data.force_update,
    })
}

fn parse_semver(value: &str, label: &str) -> Result<semver::Version, String> {
    let trimmed = value.trim().trim_start_matches(['v', 'V']);
    semver::Version::parse(trimmed)
        .map_err(|error| format!("Invalid {label} version {value}: {error}"))
}

pub(super) fn latest_download_url(data: &ClientUpdateData) -> Result<String, String> {
    let url = data.download_url.trim().to_string();
    if url.is_empty() {
        Err("Update service did not provide a download URL.".into())
    } else {
        Ok(url)
    }
}

fn apply_latest_package(
    pending: &mut PendingUpdate,
    data: &ClientUpdateData,
) -> Result<String, String> {
    let download_url = latest_download_url(data)?;
    if pending.download_url != download_url {
        pending.local_path = None;
    }
    pending.download_url = download_url.clone();
    pending.checksum = data.checksum.clone();
    pending.package_name = data.package_name.clone();
    if let Ok(check) = update_check_from_api(&pending.current_version, data.clone()) {
        pending.latest_version = check.latest_version;
        pending.notes = check.notes;
        pending.force_update = check.force_update;
    }
    Ok(download_url)
}

async fn fetch_client_update_data() -> Result<ClientUpdateData, String> {
    let current_version = env!("CARGO_PKG_VERSION");
    let url = format!(
        "{}{CHECK_PATH}?platform={}&version={}&arch={}",
        crate::tctoken::tctoken_api_base(),
        client_update_platform(),
        current_version,
        client_update_arch()
    );

    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|error| format!("Failed to create update client: {error}"))?
        .get(&url)
        .header(reqwest::header::USER_AGENT, "superscience-update-check")
        .send()
        .await
        .map_err(|error| format!("Failed to check for updates: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Update service returned an error: {error}"))?;

    let envelope = response
        .json::<ApiEnvelope<ClientUpdateData>>()
        .await
        .map_err(|error| format!("Invalid response from update service: {error}"))?;
    if !envelope.success {
        let message = envelope.message.trim();
        return Err(if message.is_empty() {
            "Update service reported a failure.".into()
        } else {
            message.to_string()
        });
    }
    envelope
        .data
        .ok_or_else(|| "Update service returned no data.".to_string())
}

pub(super) fn normalize_sha256_checksum(value: &str) -> Result<Option<String>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let hex = trimmed
        .strip_prefix("sha256:")
        .or_else(|| trimmed.strip_prefix("SHA256:"))
        .unwrap_or(trimmed)
        .trim();
    if hex.len() != 64 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err("Update checksum must be a 64-character SHA-256 hex digest.".into());
    }
    Ok(Some(hex.to_ascii_lowercase()))
}

fn package_file_name(package_name: &str, download_url: &str) -> String {
    let from_api = PathBuf::from(package_name.trim())
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && *name != "." && *name != "..")
        .map(ToOwned::to_owned);
    if let Some(name) = from_api {
        return name;
    }
    url::Url::parse(download_url)
        .ok()
        .and_then(|parsed| {
            parsed
                .path_segments()
                .and_then(|mut parts| parts.next_back())
                .map(ToOwned::to_owned)
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "client-update.bin".into())
}

#[tauri::command]
pub(super) async fn check_for_updates(
    pending: State<'_, PendingAppUpdate>,
) -> Result<UpdateCheck, String> {
    if let Some(check) = pending
        .0
        .lock()
        .unwrap()
        .as_ref()
        .filter(|update| update.downloading || update.local_path.is_some())
        .map(PendingUpdate::check)
    {
        return Ok(check);
    }

    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let data = fetch_client_update_data().await?;
    let check = update_check_from_api(&current_version, data.clone())?;

    if check.update_available {
        *pending.0.lock().unwrap() = Some(PendingUpdate {
            current_version: check.current_version.clone(),
            latest_version: check.latest_version.clone(),
            download_url: data.download_url.trim().to_string(),
            checksum: data.checksum,
            package_name: data.package_name,
            notes: check.notes.clone(),
            force_update: check.force_update,
            local_path: None,
            downloading: false,
        });
    } else {
        *pending.0.lock().unwrap() = None;
    }
    let _ = (&data.platform, data.file_size);
    Ok(check)
}

#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub(super) enum UpdateDownloadEvent {
    Started {
        content_length: Option<u64>,
    },
    Progress {
        chunk_length: u64,
    },
    /// Emitted only after the package has been written and checksum-verified.
    Verified,
}

#[tauri::command]
pub(super) async fn download_update(
    pending: State<'_, PendingAppUpdate>,
    on_event: Channel<UpdateDownloadEvent>,
) -> Result<(), String> {
    {
        let pending = pending.0.lock().unwrap();
        let update = pending
            .as_ref()
            .ok_or_else(|| "Check for updates before downloading.".to_string())?;
        if update.downloading {
            return Err("This update is already downloading in another window.".into());
        }
    }

    match fetch_client_update_data().await {
        Ok(data) => {
            let mut pending = pending.0.lock().unwrap();
            let update = pending
                .as_mut()
                .ok_or_else(|| "Check for updates before downloading.".to_string())?;
            if update.downloading {
                return Err("This update is already downloading in another window.".into());
            }
            apply_latest_package(update, &data)?;
        }
        Err(error) => {
            let pending = pending.0.lock().unwrap();
            let has_cached = pending
                .as_ref()
                .is_some_and(|update| !update.download_url.trim().is_empty());
            if !has_cached {
                return Err(error);
            }
            tracing::warn!("latest update lookup failed, using cached download URL: {error}");
        }
    }

    let (download_url, checksum, file_name) = {
        let mut pending = pending.0.lock().unwrap();
        let update = pending
            .as_mut()
            .ok_or_else(|| "Check for updates before downloading.".to_string())?;
        if update
            .local_path
            .as_ref()
            .is_some_and(|path| path.is_file())
        {
            let _ = on_event.send(UpdateDownloadEvent::Verified);
            return Ok(());
        }
        if update.downloading {
            return Err("This update is already downloading in another window.".into());
        }
        if update.download_url.trim().is_empty() {
            return Err("Update service did not provide a download URL.".into());
        }
        update.downloading = true;
        (
            update.download_url.clone(),
            update.checksum.clone(),
            package_file_name(&update.package_name, &update.download_url),
        )
    };

    let expected = match normalize_sha256_checksum(&checksum) {
        Ok(value) => value,
        Err(error) => {
            if let Some(update) = pending.0.lock().unwrap().as_mut() {
                update.downloading = false;
            }
            return Err(error);
        }
    };

    let result = download_package(&download_url, &file_name, expected.as_deref(), &on_event).await;

    match result {
        Ok(path) => {
            if let Some(update) = pending.0.lock().unwrap().as_mut() {
                update.local_path = Some(path);
                update.downloading = false;
            }
            let _ = on_event.send(UpdateDownloadEvent::Verified);
            Ok(())
        }
        Err(error) => {
            if let Some(update) = pending.0.lock().unwrap().as_mut() {
                update.downloading = false;
            }
            Err(error)
        }
    }
}

async fn download_package(
    download_url: &str,
    file_name: &str,
    expected_sha256: Option<&str>,
    on_event: &Channel<UpdateDownloadEvent>,
) -> Result<PathBuf, String> {
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15 * 60))
        .build()
        .map_err(|error| format!("Failed to create download client: {error}"))?
        .get(download_url)
        .header(reqwest::header::USER_AGENT, "superscience-update-check")
        .send()
        .await
        .map_err(|error| format!("Failed to download update: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Update download returned an error: {error}"))?;

    let content_length = response.content_length();
    let _ = on_event.send(UpdateDownloadEvent::Started { content_length });

    let path = std::env::temp_dir().join(format!(
        "superscience-update-{}-{file_name}",
        uuid::Uuid::new_v4()
    ));
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|error| format!("Failed to create update file: {error}"))?;
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("Update download failed: {error}"))?;
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|error| format!("Failed to write update file: {error}"))?;
        let _ = on_event.send(UpdateDownloadEvent::Progress {
            chunk_length: chunk.len() as u64,
        });
    }
    file.flush()
        .await
        .map_err(|error| format!("Failed to finish update file: {error}"))?;

    if let Some(expected) = expected_sha256 {
        let actual = hex::encode(hasher.finalize());
        if actual != expected {
            let _ = tokio::fs::remove_file(&path).await;
            return Err(format!(
                "Update checksum mismatch (expected {expected}, got {actual})."
            ));
        }
    } else {
        tracing::info!("update package checksum omitted by server; skipped SHA-256 verification");
    }
    Ok(path)
}

#[tauri::command]
pub(super) async fn install_update(
    app: AppHandle,
    pending: State<'_, PendingAppUpdate>,
) -> Result<(), String> {
    let path = pending
        .0
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|update| update.local_path.clone())
        .ok_or_else(|| {
            "Download and verify the update before opening the installer.".to_string()
        })?;
    if !path.is_file() {
        return Err("The downloaded update package is missing. Download it again.".into());
    }
    app.opener()
        .open_path(path.to_string_lossy().as_ref(), None::<&str>)
        .map_err(|error| format!("Failed to open the update package: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> ClientUpdateData {
        ClientUpdateData {
            has_update: Some(true),
            force_update: false,
            current_version: "1.1.0".into(),
            latest_version: "1.2.0".into(),
            package_name: "天成科研助手_1.2.0_aarch64.dmg".into(),
            download_url: "https://www.tctoken.cn/app/tiancheng.dmg".into(),
            release_notes: "修复登录过期提示".into(),
            checksum: "a".repeat(64),
            file_size: 12,
            platform: "macos-m".into(),
        }
    }

    #[test]
    fn api_payload_maps_to_available_update() {
        let check = update_check_from_api("1.1.0", sample_data()).unwrap();
        assert!(check.update_available);
        assert!(check.install_supported);
        assert!(!check.force_update);
        assert_eq!(check.latest_version, "1.2.0");
        assert_eq!(
            check.release_url,
            "https://www.tctoken.cn/app/tiancheng.dmg"
        );
        assert_eq!(check.notes, "修复登录过期提示");
    }

    #[test]
    fn api_envelope_json_maps_to_update_check() {
        let envelope: ApiEnvelope<ClientUpdateData> = serde_json::from_str(
            r#"{
                "success": true,
                "message": "ok",
                "data": {
                    "has_update": true,
                    "force_update": true,
                    "current_version": "1.1.0",
                    "latest_version": "v1.3.0",
                    "package_name": "tiancheng.dmg",
                    "download_url": "https://www.tctoken.cn/app/tiancheng.dmg",
                    "release_notes": "notes",
                    "checksum": "",
                    "file_size": 42
                }
            }"#,
        )
        .unwrap();
        assert!(envelope.success);
        let check = update_check_from_api("1.1.0", envelope.data.unwrap()).unwrap();
        assert!(check.update_available);
        assert!(check.force_update);
        assert_eq!(check.latest_version, "1.3.0");
        assert_eq!(
            check.release_url,
            "https://www.tctoken.cn/app/tiancheng.dmg"
        );
    }

    #[test]
    fn server_has_update_does_not_downgrade() {
        let mut data = sample_data();
        data.has_update = Some(true);
        data.latest_version = "1.0.0".into();
        let check = update_check_from_api("1.2.0", data).unwrap();
        assert!(!check.update_available);
        assert!(!check.install_supported);
        assert!(!check.force_update);
    }

    #[test]
    fn force_update_only_applies_when_newer() {
        let mut data = sample_data();
        data.force_update = true;
        let check = update_check_from_api("1.1.0", data).unwrap();
        assert!(check.force_update);

        let mut same = sample_data();
        same.force_update = true;
        same.has_update = Some(false);
        same.latest_version = "1.1.0".into();
        let check = update_check_from_api("1.1.0", same).unwrap();
        assert!(!check.force_update);
    }

    #[test]
    fn empty_download_url_disables_in_app_download() {
        let mut data = sample_data();
        data.download_url.clear();
        let check = update_check_from_api("1.1.0", data.clone()).unwrap();
        assert!(check.update_available);
        assert!(!check.install_supported);
        assert!(latest_download_url(&data).is_err());
    }

    #[test]
    fn latest_download_url_uses_api_package_even_without_has_update() {
        let mut data = sample_data();
        data.has_update = Some(false);
        data.download_url = "https://www.tctoken.cn/app/tiancheng-1.9.0.dmg".into();
        assert_eq!(
            latest_download_url(&data).unwrap(),
            "https://www.tctoken.cn/app/tiancheng-1.9.0.dmg"
        );
    }

    #[test]
    fn apply_latest_package_replaces_stale_download_url() {
        let mut pending = PendingUpdate {
            current_version: "1.1.0".into(),
            latest_version: "1.2.0".into(),
            download_url: "https://www.tctoken.cn/app/old.dmg".into(),
            checksum: String::new(),
            package_name: "old.dmg".into(),
            notes: String::new(),
            force_update: false,
            local_path: Some(PathBuf::from("/tmp/old.dmg")),
            downloading: false,
        };
        let mut data = sample_data();
        data.latest_version = "1.3.0".into();
        data.download_url = "https://www.tctoken.cn/app/tiancheng-1.3.0.dmg".into();
        data.package_name = "tiancheng-1.3.0.dmg".into();
        let url = apply_latest_package(&mut pending, &data).unwrap();
        assert_eq!(url, "https://www.tctoken.cn/app/tiancheng-1.3.0.dmg");
        assert_eq!(pending.latest_version, "1.3.0");
        assert_eq!(pending.package_name, "tiancheng-1.3.0.dmg");
        assert!(pending.local_path.is_none());
    }

    #[test]
    fn checksum_normalizes_sha256_prefix_and_rejects_junk() {
        assert_eq!(normalize_sha256_checksum("").unwrap(), None);
        assert_eq!(
            normalize_sha256_checksum(&format!("sha256:{}", "B".repeat(64))).unwrap(),
            Some("b".repeat(64))
        );
        assert!(normalize_sha256_checksum("not-a-hash").is_err());
    }

    #[test]
    fn package_name_uses_basename_only() {
        assert_eq!(
            package_file_name("../evil.dmg", "https://www.tctoken.cn/app/good.dmg"),
            "evil.dmg"
        );
        assert_eq!(
            package_file_name("", "https://www.tctoken.cn/app/fallback.dmg"),
            "fallback.dmg"
        );
    }

    #[test]
    fn platform_and_arch_are_api_enums() {
        let platform = client_update_platform();
        assert!(matches!(platform, "macos" | "windows" | "linux"));
        let arch = client_update_arch();
        assert!(matches!(arch, "m" | "intel" | "arm64" | "x64"));
        assert_eq!(client_update_arch_for("macos", "aarch64"), "m");
        assert_eq!(client_update_arch_for("macos", "x86_64"), "intel");
        assert_eq!(client_update_arch_for("windows", "aarch64"), "arm64");
        assert_eq!(client_update_arch_for("linux", "x86_64"), "x64");
    }

    #[test]
    fn missing_has_update_uses_semver() {
        let envelope: ApiEnvelope<ClientUpdateData> = serde_json::from_str(
            r#"{
                "success": true,
                "message": "",
                "data": {
                    "platform": "macos-m",
                    "force_update": false,
                    "current_version": "1.1.0",
                    "latest_version": "1.2.0",
                    "package_name": "TCToken-arm64.dmg",
                    "download_url": "https://www.tctoken.cn/downloads/TCToken-arm64.dmg",
                    "release_notes": "",
                    "checksum": "",
                    "file_size": 0
                }
            }"#,
        )
        .unwrap();
        let data = envelope.data.unwrap();
        assert_eq!(data.has_update, None);
        assert_eq!(data.platform, "macos-m");
        let check = update_check_from_api("1.1.0", data).unwrap();
        assert!(check.update_available);
        assert!(!check.force_update);
    }

    #[test]
    fn explicit_has_update_false_respects_server() {
        let mut data = sample_data();
        data.has_update = Some(false);
        data.latest_version = "1.9.0".into();
        let check = update_check_from_api("1.1.0", data).unwrap();
        assert!(!check.update_available);
        assert!(!check.force_update);
    }

    #[test]
    fn force_update_does_not_downgrade() {
        let mut data = sample_data();
        data.has_update = None;
        data.force_update = true;
        data.latest_version = "1.0.0".into();
        let check = update_check_from_api("1.2.0", data).unwrap();
        assert!(!check.update_available);
        assert!(!check.force_update);
    }

    #[test]
    fn download_events_keep_the_frontend_channel_shape() {
        assert_eq!(
            serde_json::to_value(UpdateDownloadEvent::Started {
                content_length: Some(42)
            })
            .unwrap(),
            serde_json::json!({
                "event": "started",
                "data": { "content_length": 42 }
            })
        );
    }
}
