//! Secret storage for API keys and credentials.
//!
//! Secrets live in a local JSON file (mode `0600` on Unix) under the app data
//! directory. This avoids macOS Keychain ACL prompts when the app is rebuilt or
//! updated (Keychain items are bound to the calling app's code signature).
//!
//! Override the file location with `SUPERSCIENCE_SECRETS_FILE` (legacy
//! `WISP_SECRETS_FILE` is also accepted). There is no OS-keyring backend: all
//! installs are expected to use this file store.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

/// A named secret (e.g. an API key).
pub struct Secret;

impl Secret {
    pub fn set(name: &str, value: &str) -> anyhow::Result<()> {
        file::set(name, value)
    }

    pub fn get(name: &str) -> anyhow::Result<String> {
        file::get(name)
    }

    pub fn delete(name: &str) -> anyhow::Result<()> {
        file::delete(name)
    }

    /// Resolved secrets file path (for diagnostics / docs).
    pub fn file_path() -> PathBuf {
        file::path()
    }

    /// Active backend name. Always `"file"`.
    pub fn backend_name() -> &'static str {
        "file"
    }
}

fn env_var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

mod file {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::Mutex;

    fn lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    pub fn path() -> PathBuf {
        if let Some(custom) =
            env_var("SUPERSCIENCE_SECRETS_FILE").or_else(|| env_var("WISP_SECRETS_FILE"))
        {
            return PathBuf::from(custom);
        }
        #[cfg(debug_assertions)]
        {
            // Keep the historic debug path so existing dev keys keep working.
            if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
            {
                return PathBuf::from(home).join(".superscience-dev-secrets.json");
            }
        }
        if let Some(dir) = dirs::data_dir() {
            return dir
                .join("science.superscience")
                .join("superscience")
                .join("secrets.json");
        }
        std::env::temp_dir().join("superscience-secrets.json")
    }

    fn load() -> BTreeMap<String, String> {
        fs::read(path())
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn store(map: &BTreeMap<String, String>) -> anyhow::Result<()> {
        let path = path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(map)?;
        let mut file = fs::File::create(&path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    pub fn set(name: &str, value: &str) -> anyhow::Result<()> {
        let _guard = lock()
            .lock()
            .map_err(|_| anyhow::anyhow!("secrets file lock poisoned"))?;
        let mut map = load();
        map.insert(name.to_string(), value.to_string());
        store(&map)
    }

    pub fn get(name: &str) -> anyhow::Result<String> {
        let _guard = lock()
            .lock()
            .map_err(|_| anyhow::anyhow!("secrets file lock poisoned"))?;
        load()
            .remove(name)
            .ok_or_else(|| anyhow::anyhow!("no secret named {name}"))
    }

    pub fn delete(name: &str) -> anyhow::Result<()> {
        let _guard = lock()
            .lock()
            .map_err(|_| anyhow::anyhow!("secrets file lock poisoned"))?;
        let mut map = load();
        map.remove(name);
        store(&map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialize tests that mutate process-global env / OnceLock-backed paths.
    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn file_backend_roundtrip() {
        let _guard = test_lock();
        let path = std::env::temp_dir().join(format!(
            "superscience-secrets-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::env::set_var("SUPERSCIENCE_SECRETS_FILE", &path);
        // Exercise the file module directly so OnceLock backend selection from
        // other tests in this process cannot interfere.
        file::set("test:roundtrip", "abc123").unwrap();
        assert_eq!(file::get("test:roundtrip").unwrap(), "abc123");
        file::delete("test:roundtrip").unwrap();
        assert!(file::get("test:roundtrip").is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        let _ = std::fs::remove_file(&path);
        std::env::remove_var("SUPERSCIENCE_SECRETS_FILE");
    }

    #[test]
    fn backend_name_is_file() {
        assert_eq!(Secret::backend_name(), "file");
    }
}
