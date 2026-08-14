//! Model profiles: several named LLM configs (provider + API URL + model +
//! its own key), one of them active. The active profile drives every turn —
//! `load_settings` resolves through here — and the composer switches it.
//!
//! Legacy single-model installs are migrated into one "default" profile the
//! first time this is read, so nothing breaks and no key is lost.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tauri::State;

pub const DEFAULT_CONTEXT_WINDOW: u64 = 128_000;

fn default_context_window() -> u64 {
    DEFAULT_CONTEXT_WINDOW
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub id: String,
    pub label: String,
    /// Protocol type (`openai` / `openai_responses` / `anthropic`). Mirrored
    /// from the owning provider on read so callers keep working.
    pub provider: String,
    /// API base URL mirrored from the owning provider on read.
    pub api_url: String,
    /// Owning provider id. Empty only for pre-migration JSON; `ensure` fills it.
    #[serde(default)]
    pub provider_id: String,
    pub model: String,
    /// Computed on read from the secrets file; never part of the persisted JSON.
    #[serde(default)]
    pub has_api_key: bool,
    /// Computed on read; true for the active profile.
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub max_tokens: u64,
    /// Total input + output context capacity advertised for this model. Reader
    /// session splitting uses this value; it is not sent to the provider.
    #[serde(default = "default_context_window")]
    pub context_window: u64,
    #[serde(default)]
    pub reasoning_effort: String,
    /// Capability marker: this API model can accept image input.
    #[serde(default)]
    pub supports_vision: bool,
    /// Computed on read / accepted on save; true when this profile is assigned
    /// to image analysis. Serialized so the UI can restore the checkbox.
    #[serde(default)]
    pub use_for_vision: bool,
    /// Computed on read / accepted on save; true when this profile is assigned
    /// to the Scientific Illustrator's raster image-generation tool.
    #[serde(default)]
    pub use_for_image_generation: bool,
}

/// Shared endpoint + credentials for one or more [`ModelProfile`]s.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProvider {
    pub id: String,
    pub label: String,
    /// Protocol: `openai` | `openai_responses` | `anthropic`.
    pub protocol: String,
    pub api_url: String,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub builtin: bool,
    /// Computed on read from the secrets file; never persisted.
    #[serde(default)]
    pub has_api_key: bool,
    /// Computed on read: number of model profiles under this provider.
    #[serde(default)]
    pub model_count: usize,
}

const PROFILES_KEY: &str = "model_profiles";
const PROVIDERS_KEY: &str = "model_providers";
const ACTIVE_KEY: &str = "active_model_id";
const VISION_KEY: &str = "vision_model_id";
const IMAGE_GENERATION_KEY: &str = "image_generation_model_id";
const LEGACY_KEY_SECRET: &str = "api_key";
const CUSTOM_CREDENTIALS_KEY: &str = "custom_credentials";
const CUSTOM_CREDENTIAL_SECRET_PREFIX: &str = "custom_credential:";
pub const TCTOKEN_PROVIDER_ID: &str = "tctoken";
const TCTOKEN_PROVIDER_LABEL: &str = "天成TOKEN平台";
const TCTOKEN_PROVIDER_URL: &str = "https://www.tctoken.cn/v1";

fn secret_name(id: &str) -> String {
    format!("model_key:{id}")
}

fn provider_secret_name(id: &str) -> String {
    format!("provider_key:{id}")
}

/// Process-lifetime cache of resolved secrets, keyed by secret name.
///
/// Avoids re-reading the secrets file on every UI refresh. Writes go through
/// `secret_set`/`secret_del` so the cache never goes stale. Values are dropped
/// on process exit.
fn secret_cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn secret_get(name: &str) -> String {
    if let Some(v) = secret_cache().lock().unwrap().get(name) {
        return v.clone();
    }
    let v = superscience_store::secrets::Secret::get(name)
        .ok()
        .unwrap_or_default();
    secret_cache()
        .lock()
        .unwrap()
        .insert(name.to_string(), v.clone());
    v
}

fn secret_set(name: &str, value: &str) -> Result<(), String> {
    superscience_store::secrets::Secret::set(name, value).map_err(|e| e.to_string())?;
    secret_cache()
        .lock()
        .unwrap()
        .insert(name.to_string(), value.to_string());
    Ok(())
}

fn secret_del(name: &str) -> Result<(), String> {
    let r = superscience_store::secrets::Secret::delete(name).map_err(|e| e.to_string());
    // Remember "absent" so existence checks don't re-read the secrets file.
    secret_cache()
        .lock()
        .unwrap()
        .insert(name.to_string(), String::new());
    r
}

/// Service credentials (#115): API keys/emails for external services that
/// skills and bundled MCP tools authenticate to. Each is stored in the local
/// secrets file (same cache as model keys, read at most once per launch) and
/// injected as an env var into spawned Python/MCP processes. `id` is the
/// stable UI/command identifier; `secret` is the secrets-file key; `env` is the
/// variable the consuming Python reads.
struct Credential {
    id: &'static str,
    secret: &'static str,
    env: &'static str,
}

/// User-defined credential metadata. The value is deliberately absent: only
/// this non-secret name/environment mapping is persisted in SQLite, while the
/// value stays in the local secrets file under an id-derived entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomCredential {
    pub id: String,
    pub name: String,
    pub env_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomCredentialStatus {
    pub id: String,
    pub name: String,
    pub env_var: String,
    pub present: bool,
}

const CREDENTIALS: &[Credential] = &[
    Credential {
        id: "openalex_api_key",
        secret: "openalex_api_key",
        env: "OPENALEX_API_KEY",
    },
    Credential {
        id: "infinisynapse_api_key",
        secret: "infinisynapse_api_key",
        env: "INFINISYNAPSE_API_KEY",
    },
    Credential {
        id: "scimaster_api_key",
        secret: "scimaster_api_key",
        env: "SCIMASTER_API_KEY",
    },
    Credential {
        id: "ncbi_api_key",
        secret: "ncbi_api_key",
        env: "NCBI_API_KEY",
    },
    Credential {
        id: "ncbi_email",
        secret: "ncbi_email",
        env: "NCBI_EMAIL",
    },
];

fn credential(id: &str) -> Option<&'static Credential> {
    CREDENTIALS.iter().find(|c| c.id == id)
}

fn custom_credentials_cache() -> &'static Mutex<Vec<CustomCredential>> {
    static CACHE: OnceLock<Mutex<Vec<CustomCredential>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Vec::new()))
}

fn custom_secret_name(id: &str) -> String {
    format!("{CUSTOM_CREDENTIAL_SECRET_PREFIX}{id}")
}

fn valid_env_var(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('_' | 'A'..='Z' | 'a'..='z'))
        && chars.all(|ch| matches!(ch, '_' | 'A'..='Z' | 'a'..='z' | '0'..='9'))
}

fn validate_custom_credential(name: &str, env_var: &str, value: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Credential name is required.".into());
    }
    if name.len() > 80 {
        return Err("Credential name must be 80 characters or fewer.".into());
    }
    if env_var.is_empty() {
        return Err("Environment variable is required.".into());
    }
    if env_var.len() > 128 || !valid_env_var(env_var) {
        return Err(
            "Environment variable must start with a letter or underscore and contain only letters, numbers, and underscores."
                .into(),
        );
    }
    if value.is_empty() {
        return Err("Credential value is required.".into());
    }
    Ok(())
}

fn sanitized_custom_credentials(raw: &str) -> Vec<CustomCredential> {
    let mut ids = std::collections::HashSet::new();
    let mut env_vars = std::collections::HashSet::new();
    serde_json::from_str::<Vec<CustomCredential>>(raw)
        .unwrap_or_default()
        .into_iter()
        .filter(|credential| {
            let env_key = credential.env_var.to_ascii_uppercase();
            uuid::Uuid::parse_str(&credential.id).is_ok()
                && !credential.name.trim().is_empty()
                && valid_env_var(&credential.env_var)
                && ids.insert(credential.id.clone())
                && env_vars.insert(env_key)
        })
        .collect()
}

/// Load user-defined credential metadata from SQLite into the synchronous
/// process cache used by runtime/MCP launch paths.
pub async fn load_custom_credentials(
    store: &superscience_store::Store,
) -> Result<Vec<CustomCredential>, String> {
    let raw = store
        .get_setting(CUSTOM_CREDENTIALS_KEY)
        .await
        .map_err(|error| error.to_string())?
        .unwrap_or_default();
    let credentials = sanitized_custom_credentials(&raw);
    *custom_credentials_cache().lock().unwrap() = credentials.clone();
    Ok(credentials)
}

async fn save_custom_credentials(
    store: &superscience_store::Store,
    credentials: &[CustomCredential],
) -> Result<(), String> {
    let raw = serde_json::to_string(credentials).map_err(|error| error.to_string())?;
    store
        .set_setting(CUSTOM_CREDENTIALS_KEY, &raw)
        .await
        .map_err(|error| error.to_string())?;
    *custom_credentials_cache().lock().unwrap() = credentials.to_vec();
    Ok(())
}

pub async fn custom_credential_status(
    store: &superscience_store::Store,
) -> Result<Vec<CustomCredentialStatus>, String> {
    Ok(load_custom_credentials(store)
        .await?
        .into_iter()
        .map(|credential| CustomCredentialStatus {
            present: !secret_get(&custom_secret_name(&credential.id)).is_empty(),
            id: credential.id,
            name: credential.name,
            env_var: credential.env_var,
        })
        .collect())
}

pub async fn add_custom_credential(
    store: &superscience_store::Store,
    name: &str,
    env_var: &str,
    value: &str,
) -> Result<CustomCredentialStatus, String> {
    let name = name.trim();
    let env_var = env_var.trim();
    let value = value.trim();
    validate_custom_credential(name, env_var, value)?;

    let mut credentials = load_custom_credentials(store).await?;
    if CREDENTIALS
        .iter()
        .any(|credential| credential.env.eq_ignore_ascii_case(env_var))
    {
        return Err(format!(
            "A credential already uses environment variable {env_var}."
        ));
    }

    // Re-adding an env var that already has a row overwrites it in place, so a
    // cleared or lost value never blocks reconfiguration (#335).
    if let Some(existing) = credentials
        .iter_mut()
        .find(|credential| credential.env_var.eq_ignore_ascii_case(env_var))
    {
        existing.name = name.to_string();
        let credential = existing.clone();
        secret_set(&custom_secret_name(&credential.id), value)?;
        save_custom_credentials(store, &credentials).await?;
        return Ok(CustomCredentialStatus {
            id: credential.id,
            name: credential.name,
            env_var: credential.env_var,
            present: true,
        });
    }

    let credential = CustomCredential {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        env_var: env_var.to_string(),
    };
    let secret_name = custom_secret_name(&credential.id);
    secret_set(&secret_name, value)?;
    credentials.push(credential.clone());
    if let Err(error) = save_custom_credentials(store, &credentials).await {
        let _ = secret_del(&secret_name);
        return Err(error);
    }
    Ok(CustomCredentialStatus {
        id: credential.id,
        name: credential.name,
        env_var: credential.env_var,
        present: true,
    })
}

pub async fn remove_custom_credential(
    store: &superscience_store::Store,
    id: &str,
) -> Result<(), String> {
    let mut credentials = load_custom_credentials(store).await?;
    let index = credentials
        .iter()
        .position(|credential| credential.id == id)
        .ok_or_else(|| format!("unknown custom credential: {id}"))?;
    let secret_name = custom_secret_name(id);
    if !secret_get(&secret_name).is_empty() {
        secret_del(&secret_name)?;
    }
    credentials.remove(index);
    save_custom_credentials(store, &credentials).await
}

/// `(id, present)` for every known credential, for the Settings UI.
pub fn credential_status() -> Vec<(String, bool)> {
    let mut status = CREDENTIALS
        .iter()
        .map(|c| (c.id.to_string(), !secret_get(c.secret).is_empty()))
        .collect::<Vec<_>>();
    status.extend(custom_credentials_cache().lock().unwrap().iter().map(|c| {
        (
            c.id.clone(),
            !secret_get(&custom_secret_name(&c.id)).is_empty(),
        )
    }));
    status
}

/// Store (or clear, when `value` is blank) a credential by id. Returns an
/// error for an unknown id.
pub fn store_credential(id: &str, value: &str) -> Result<(), String> {
    let secret = credential(id)
        .map(|credential| credential.secret.to_string())
        .or_else(|| {
            custom_credentials_cache()
                .lock()
                .unwrap()
                .iter()
                .find(|credential| credential.id == id)
                .map(|credential| custom_secret_name(&credential.id))
        })
        .ok_or_else(|| format!("unknown credential: {id}"))?;
    let value = value.trim();
    if value.is_empty() {
        // Clearing a never-stored key is fine — cache records "absent".
        let _ = secret_del(&secret);
        Ok(())
    } else {
        secret_set(&secret, value)
    }
}

/// Extra env vars for spawned service processes (Python REPL kernel and the
/// bundled bio-tools MCP server), so skills and literature tools can
/// authenticate to external APIs. Only set credentials are included.
pub fn service_env() -> Vec<(String, String)> {
    let mut env = CREDENTIALS
        .iter()
        .filter_map(|c| {
            let v = secret_get(c.secret);
            (!v.is_empty()).then(|| (c.env.to_string(), v))
        })
        .collect::<Vec<_>>();
    env.extend(
        custom_credentials_cache()
            .lock()
            .unwrap()
            .iter()
            .filter_map(|credential| {
                let value = secret_get(&custom_secret_name(&credential.id));
                (!value.is_empty()).then(|| (credential.env_var.clone(), value))
            }),
    );
    env
}

async fn load_raw(store: &superscience_store::Store) -> Vec<ModelProfile> {
    let Some(raw) = store.get_setting(PROFILES_KEY).await.ok().flatten() else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<ModelProfile>>(&raw).unwrap_or_default()
}

async fn save_raw(
    store: &superscience_store::Store,
    profiles: &[ModelProfile],
) -> Result<(), String> {
    let json = serde_json::to_string(profiles).map_err(|e| e.to_string())?;
    store
        .set_setting(PROFILES_KEY, &json)
        .await
        .map_err(|e| e.to_string())
}

async fn load_providers_raw(store: &superscience_store::Store) -> Vec<ModelProvider> {
    let Some(raw) = store.get_setting(PROVIDERS_KEY).await.ok().flatten() else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<ModelProvider>>(&raw).unwrap_or_default()
}

async fn save_providers_raw(
    store: &superscience_store::Store,
    providers: &[ModelProvider],
) -> Result<(), String> {
    let json = serde_json::to_string(providers).map_err(|e| e.to_string())?;
    store
        .set_setting(PROVIDERS_KEY, &json)
        .await
        .map_err(|e| e.to_string())
}

fn normalize_api_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_ascii_lowercase()
}

fn protocol_of(profile: &ModelProfile) -> String {
    let p = profile.provider.trim();
    if p.is_empty() {
        "openai".into()
    } else {
        p.to_string()
    }
}

fn fresh_provider_id(existing: &[ModelProvider]) -> String {
    for n in 1..10_000 {
        let id = format!("p{n}");
        if !existing.iter().any(|p| p.id == id) {
            return id;
        }
    }
    "p".into()
}

fn builtin_tctoken_provider() -> ModelProvider {
    ModelProvider {
        id: TCTOKEN_PROVIDER_ID.into(),
        label: TCTOKEN_PROVIDER_LABEL.into(),
        protocol: "openai".into(),
        api_url: TCTOKEN_PROVIDER_URL.into(),
        sort_order: 0,
        builtin: true,
        has_api_key: false,
        model_count: 0,
    }
}

fn strip_computed_provider(mut p: ModelProvider) -> ModelProvider {
    p.has_api_key = false;
    p.model_count = 0;
    p
}

/// Ensure at least the builtin 天成TOKEN provider exists, migrate legacy flat
/// profiles into providers, and return profiles with `provider_id` filled.
async fn ensure(store: &superscience_store::Store) -> Vec<ModelProfile> {
    let mut profiles = load_raw(store).await;
    if profiles.is_empty() {
        // Legacy single-model install → one default profile first.
        let provider = store
            .get_setting("provider")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let api_url = store
            .get_setting("api_url")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let model = store
            .get_setting("model")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let max_tokens = store
            .get_setting("max_tokens")
            .await
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let reasoning_effort = store
            .get_setting("reasoning_effort")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        if !(provider.is_empty() && api_url.is_empty() && model.is_empty()) {
            let default = ModelProfile {
                id: "default".into(),
                label: if model.trim().is_empty() {
                    "Default".into()
                } else {
                    model.clone()
                },
                provider,
                api_url,
                provider_id: String::new(),
                model,
                has_api_key: false,
                active: false,
                max_tokens,
                context_window: DEFAULT_CONTEXT_WINDOW,
                reasoning_effort,
                supports_vision: false,
                use_for_vision: false,
                use_for_image_generation: false,
            };
            profiles = vec![default];
            let _ = save_raw(store, &profiles).await;
            let _ = store.set_setting(ACTIVE_KEY, "default").await;
            let legacy = secret_get(LEGACY_KEY_SECRET);
            if !legacy.is_empty() {
                let _ = secret_set(&secret_name("default"), &legacy);
            }
        }
    }

    let mut providers = load_providers_raw(store).await;
    let mut providers_dirty = false;
    let mut profiles_dirty = false;

    if providers.is_empty() && !profiles.is_empty() {
        // Cluster flat profiles by (protocol, normalized URL).
        let mut groups: Vec<(String, String, Vec<usize>)> = Vec::new();
        for (idx, profile) in profiles.iter().enumerate() {
            let protocol = protocol_of(profile);
            let url = normalize_api_url(&profile.api_url);
            if let Some((_, _, idxs)) = groups
                .iter_mut()
                .find(|(p, u, _)| *p == protocol && *u == url)
            {
                idxs.push(idx);
            } else {
                groups.push((protocol, url, vec![idx]));
            }
        }
        for (protocol, url, idxs) in groups {
            let first = &profiles[idxs[0]];
            let id = if normalize_api_url(&first.api_url) == normalize_api_url(TCTOKEN_PROVIDER_URL)
                && protocol == "openai"
            {
                TCTOKEN_PROVIDER_ID.to_string()
            } else {
                fresh_provider_id(&providers)
            };
            let label = if id == TCTOKEN_PROVIDER_ID {
                TCTOKEN_PROVIDER_LABEL.to_string()
            } else if !first.label.trim().is_empty() {
                // Prefer a host-ish label from URL when available.
                url.split("://")
                    .nth(1)
                    .and_then(|rest| rest.split('/').next())
                    .filter(|h| !h.is_empty())
                    .map(|h| h.to_string())
                    .unwrap_or_else(|| first.label.clone())
            } else {
                "Provider".into()
            };
            let api_url = if first.api_url.trim().is_empty() {
                if id == TCTOKEN_PROVIDER_ID {
                    TCTOKEN_PROVIDER_URL.to_string()
                } else {
                    String::new()
                }
            } else {
                first.api_url.clone()
            };
            let provider = ModelProvider {
                id: id.clone(),
                label,
                protocol: protocol.clone(),
                api_url,
                sort_order: providers.len() as i64 + 1,
                builtin: id == TCTOKEN_PROVIDER_ID,
                has_api_key: false,
                model_count: 0,
            };
            // Copy first available model key onto the provider.
            for idx in &idxs {
                let k = key_for_model(&profiles[*idx].id);
                if !k.is_empty() {
                    let _ = secret_set(&provider_secret_name(&id), &k);
                    break;
                }
            }
            for idx in idxs {
                if profiles[idx].provider_id != id {
                    profiles[idx].provider_id = id.clone();
                    profiles[idx].provider = protocol.clone();
                    profiles_dirty = true;
                }
            }
            providers.push(provider);
        }
        providers_dirty = true;
    }

    // Always pin builtin 天成TOKEN at the front.
    match providers
        .iter_mut()
        .find(|p| p.id == TCTOKEN_PROVIDER_ID)
    {
        Some(existing) => {
            if !existing.builtin
                || existing.label != TCTOKEN_PROVIDER_LABEL
                || existing.protocol != "openai"
            {
                existing.builtin = true;
                existing.label = TCTOKEN_PROVIDER_LABEL.into();
                existing.protocol = "openai".into();
                if existing.api_url.trim().is_empty() {
                    existing.api_url = TCTOKEN_PROVIDER_URL.into();
                }
                providers_dirty = true;
            }
            if existing.sort_order != 0 {
                existing.sort_order = 0;
                providers_dirty = true;
            }
        }
        None => {
            providers.insert(0, builtin_tctoken_provider());
            providers_dirty = true;
        }
    }

    // Assign orphan profiles to tctoken when provider_id missing.
    for profile in &mut profiles {
        if profile.provider_id.trim().is_empty() {
            profile.provider_id = TCTOKEN_PROVIDER_ID.into();
            if profile.provider.trim().is_empty() {
                profile.provider = "openai".into();
            }
            if profile.api_url.trim().is_empty() {
                profile.api_url = TCTOKEN_PROVIDER_URL.into();
            }
            profiles_dirty = true;
        }
    }

    // Re-sort: builtin tctoken first, then by sort_order / id.
    providers.sort_by(|a, b| {
        match (a.id.as_str() == TCTOKEN_PROVIDER_ID, b.id.as_str() == TCTOKEN_PROVIDER_ID) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .sort_order
                .cmp(&b.sort_order)
                .then_with(|| a.id.cmp(&b.id)),
        }
    });
    for (idx, p) in providers.iter_mut().enumerate() {
        let want = idx as i64;
        if p.sort_order != want {
            p.sort_order = want;
            providers_dirty = true;
        }
    }

    if providers_dirty {
        let persisted: Vec<_> = providers.iter().cloned().map(strip_computed_provider).collect();
        let _ = save_providers_raw(store, &persisted).await;
    }
    if profiles_dirty {
        let _ = save_raw(store, &profiles).await;
    }

    // Mirror provider URL/protocol onto profiles for runtime callers.
    let provider_map: std::collections::HashMap<_, _> =
        providers.into_iter().map(|p| (p.id.clone(), p)).collect();
    for profile in &mut profiles {
        if let Some(prov) = provider_map.get(&profile.provider_id) {
            profile.provider = prov.protocol.clone();
            profile.api_url = prov.api_url.clone();
        }
    }
    profiles
}

async fn ensure_providers(store: &superscience_store::Store) -> Vec<ModelProvider> {
    let _ = ensure(store).await;
    load_providers_raw(store).await
}

fn key_for_model(id: &str) -> String {
    let k = secret_get(&secret_name(id));
    if k.is_empty() && id == "default" {
        secret_get(LEGACY_KEY_SECRET)
    } else {
        k
    }
}

/// Key for a profile: prefer provider-level secret, fall back to legacy model key.
fn key_for(profile: &ModelProfile) -> String {
    if !profile.provider_id.trim().is_empty() {
        let k = secret_get(&provider_secret_name(&profile.provider_id));
        if !k.is_empty() {
            return k;
        }
    }
    key_for_model(&profile.id)
}

async fn active_id(store: &superscience_store::Store, profiles: &[ModelProfile]) -> String {
    let want = store
        .get_setting(ACTIVE_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    if profiles.iter().any(|p| p.id == want && is_chat_model(p)) {
        want
    } else {
        profiles
            .iter()
            .find(|p| is_chat_model(p))
            .or_else(|| profiles.first())
            .map(|p| p.id.clone())
            .unwrap_or_default()
    }
}

pub async fn active_profile_id(store: &superscience_store::Store) -> String {
    let profiles = ensure(store).await;
    active_id(store, &profiles).await
}

pub async fn session_profile_id(store: &superscience_store::Store, frame_id: &str) -> String {
    let profiles = ensure(store).await;
    let bound = store
        .frame_model(frame_id)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    if profiles
        .iter()
        .any(|profile| profile.id == bound && is_chat_model(profile))
    {
        bound
    } else {
        active_id(store, &profiles).await
    }
}

pub async fn session_label(store: &superscience_store::Store, frame_id: &str) -> String {
    let profiles = ensure(store).await;
    let id = session_profile_id(store, frame_id).await;
    profiles
        .iter()
        .find(|profile| profile.id == id)
        .map(|profile| profile.label.clone())
        .unwrap_or_default()
}

/// Effective reasoning effort for a conversation: an explicit frame override
/// wins, otherwise the bound model profile supplies its configured default.
/// An empty stored override (written by older builds for "provider default")
/// counts as no override, so the profile default applies again.
pub async fn session_reasoning_effort(
    store: &superscience_store::Store,
    frame_id: &str,
    profile_default: &str,
) -> String {
    if let Ok(Some(effort)) = store.frame_reasoning_effort(frame_id).await {
        if !effort.is_empty() {
            return effort;
        }
    }
    profile_default.to_string()
}

/// The active profile's `(provider, api_url, model, api_key)` for a turn.
pub async fn active_config(store: &superscience_store::Store) -> (String, String, String, String) {
    let profiles = ensure(store).await;
    if profiles.is_empty() {
        return (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
    }
    let id = active_id(store, &profiles).await;
    let p = profiles
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .unwrap_or_else(|| profiles[0].clone());
    (
        p.provider.clone(),
        p.api_url.clone(),
        p.model.clone(),
        key_for(&p),
    )
}

pub(crate) fn is_image_generation_model(model: &str) -> bool {
    model.trim().eq_ignore_ascii_case("gpt-image-2")
}

pub(crate) fn supports_image_generation(provider: &str, model: &str) -> bool {
    matches!(
        provider.trim(),
        "openai" | "openai_compatible" | "openai_responses" | "openai-responses" | "responses"
    ) && is_image_generation_model(model)
}

fn is_chat_model(p: &ModelProfile) -> bool {
    !is_image_generation_model(&p.model)
}

fn can_describe_images(p: &ModelProfile) -> bool {
    is_chat_model(p) && p.supports_vision
}

fn can_generate_images(p: &ModelProfile) -> bool {
    supports_image_generation(&p.provider, &p.model)
}

async fn vision_id(store: &superscience_store::Store, profiles: &[ModelProfile]) -> Option<String> {
    let want = store
        .get_setting(VISION_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    profiles
        .iter()
        .find(|p| p.id == want && can_describe_images(p))
        .or_else(|| profiles.iter().find(|p| can_describe_images(p)))
        .map(|p| p.id.clone())
}

async fn image_generation_id(
    store: &superscience_store::Store,
    profiles: &[ModelProfile],
) -> Option<String> {
    let want = store
        .get_setting(IMAGE_GENERATION_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    profiles
        .iter()
        .find(|p| p.id == want && can_generate_images(p))
        .map(|p| p.id.clone())
}

/// The assigned vision profile's `(provider, api_url, model, api_key,
/// max_tokens, reasoning_effort)`, if the user configured one.
pub async fn vision_config(
    store: &superscience_store::Store,
) -> Option<(String, String, String, String, u64, String)> {
    let profiles = ensure(store).await;
    let id = vision_id(store, &profiles).await?;
    let p = profiles.iter().find(|p| p.id == id)?.clone();
    let key = key_for(&p);
    Some((
        p.provider,
        p.api_url,
        p.model,
        key,
        p.max_tokens,
        p.reasoning_effort,
    ))
}

/// The explicitly assigned OpenAI image profile's `(api_url, model, api_key)`.
/// Unlike vision, image generation has no implicit fallback: no assignment
/// means the Scientific Illustrator deliberately uses SVG.
pub async fn image_generation_config(
    store: &superscience_store::Store,
) -> Option<(String, String, String)> {
    let profiles = ensure(store).await;
    let id = image_generation_id(store, &profiles).await?;
    let p = profiles.iter().find(|p| p.id == id)?;
    Some((p.api_url.clone(), p.model.clone(), key_for(p)))
}

/// Replace the API key for the model assigned to image generation.
pub async fn set_image_generation_api_key(
    store: &superscience_store::Store,
    key: &str,
) -> Result<String, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err(String::from("API key is empty."));
    }
    let profiles = ensure(store).await;
    let Some(id) = image_generation_id(store, &profiles).await else {
        return Err(String::from(
            "No image-generation model is configured. Assign gpt-image-2 under Settings → Models.",
        ));
    };
    let Some(p) = profiles.iter().find(|p| p.id == id) else {
        return Err(String::from("Image-generation model profile is missing."));
    };
    if !p.provider_id.trim().is_empty() {
        secret_set(&provider_secret_name(&p.provider_id), key)?;
    } else {
        secret_set(&secret_name(&id), key)?;
    }
    Ok(id)
}

/// Update the active profile's provider/api_url/model/label. The classic Settings
/// form now edits whichever model is active, rather than a single global config.
pub async fn set_active_fields(
    store: &superscience_store::Store,
    provider: &str,
    api_url: &str,
    model: &str,
    label: &str,
) -> Result<(), String> {
    let mut profiles = ensure(store).await;
    let id = active_id(store, &profiles).await;
    if let Some(p) = profiles.iter_mut().find(|p| p.id == id) {
        p.provider = provider.to_string();
        p.api_url = api_url.to_string();
        p.model = model.to_string();
        let alias = label.trim();
        p.label = if alias.is_empty() {
            model.to_string()
        } else {
            alias.to_string()
        };
    }
    save_raw(store, &profiles).await
}

/// Display alias for the active profile (shown in the composer picker).
pub async fn active_label(store: &superscience_store::Store) -> String {
    let profiles = ensure(store).await;
    let id = active_id(store, &profiles).await;
    profiles
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.label.clone())
        .unwrap_or_default()
}

/// Per-model advanced LLM options for the active profile, falling back to
/// legacy global store keys when a profile has no values yet.
pub async fn active_llm_advanced(store: &superscience_store::Store) -> (u64, String) {
    let profiles = ensure(store).await;
    let id = active_id(store, &profiles).await;
    if let Some(p) = profiles.iter().find(|p| p.id == id) {
        let mut max_tokens = p.max_tokens;
        let mut reasoning_effort = p.reasoning_effort.clone();
        if max_tokens == 0 {
            max_tokens = store
                .get_setting("max_tokens")
                .await
                .ok()
                .flatten()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
        if reasoning_effort.is_empty() {
            reasoning_effort = store
                .get_setting("reasoning_effort")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
        }
        return (max_tokens, reasoning_effort);
    }
    let max_tokens = store
        .get_setting("max_tokens")
        .await
        .ok()
        .flatten()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let reasoning_effort = store
        .get_setting("reasoning_effort")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    (max_tokens, reasoning_effort)
}

fn effective_context_window(profile: &ModelProfile) -> u64 {
    let value = if profile.context_window >= 4_096 {
        profile.context_window
    } else {
        DEFAULT_CONTEXT_WINDOW
    };
    // Catalog ceiling: an over-declared window defeats compaction, so clamp to
    // the model's documented limit (exact id match; unknown models untouched).
    match crate::model_catalog::lookup(&profile.provider, &profile.api_url, &profile.model) {
        Some(entry) => value.min(entry.c),
        None => value,
    }
}

/// Clamp `context_window`/`max_tokens` to the model's catalog ceilings.
/// `max_tokens = 0` means "unset" and is left alone.
fn clamp_to_catalog(profile: &mut ModelProfile) {
    if let Some(entry) =
        crate::model_catalog::lookup(&profile.provider, &profile.api_url, &profile.model)
    {
        profile.context_window = profile.context_window.min(entry.c);
        profile.max_tokens = profile.max_tokens.min(entry.o);
    }
}

/// Context capacity for the active HTTP model.
pub async fn active_context_window(store: &superscience_store::Store) -> u64 {
    let profiles = ensure(store).await;
    let id = active_id(store, &profiles).await;
    profiles
        .iter()
        .find(|profile| profile.id == id)
        .map(effective_context_window)
        .unwrap_or(DEFAULT_CONTEXT_WINDOW)
}

/// Context capacity for a concrete HTTP model profile.
pub async fn profile_context_window(store: &superscience_store::Store, id: &str) -> Option<u64> {
    ensure(store)
        .await
        .iter()
        .find(|profile| profile.id == id && is_chat_model(profile))
        .map(effective_context_window)
}

/// Full LLM config for one profile id: (provider, api_url, model, api_key,
/// max_tokens, reasoning_effort). None when the id doesn't exist.
pub async fn profile_llm(
    store: &superscience_store::Store,
    id: &str,
) -> Option<(String, String, String, String, u64, String)> {
    let profiles = ensure(store).await;
    let p = profiles.iter().find(|p| p.id == id)?;
    if !is_chat_model(p) {
        return None;
    }
    Some((
        p.provider.clone(),
        p.api_url.clone(),
        p.model.clone(),
        key_for(p),
        p.max_tokens,
        p.reasoning_effort.clone(),
    ))
}

/// Stored key for a specific profile id, or None when the profile does not
/// exist. The returned string may still be empty when the profile has no key.
pub async fn profile_key(store: &superscience_store::Store, id: &str) -> Option<String> {
    let profiles = ensure(store).await;
    profiles
        .iter()
        .find(|p| p.id == id)
        .map(key_for)
}

/// Whether the active profile has a key stored (for `get_settings`).
pub async fn active_has_key(store: &superscience_store::Store) -> bool {
    let profiles = ensure(store).await;
    let id = active_id(store, &profiles).await;
    profiles
        .iter()
        .find(|p| p.id == id)
        .is_some_and(|p| !key_for(p).is_empty())
}

pub async fn active_supports_vision(store: &superscience_store::Store) -> bool {
    supports_vision(store, None).await
}

pub async fn supports_vision(store: &superscience_store::Store, profile_id: Option<&str>) -> bool {
    let profiles = ensure(store).await;
    let id = match profile_id.filter(|id| profiles.iter().any(|profile| profile.id == *id)) {
        Some(id) => id.to_string(),
        None => active_id(store, &profiles).await,
    };
    profiles
        .iter()
        .find(|p| p.id == id)
        .is_some_and(can_describe_images)
}

/// Profiles with `has_api_key`/`active` filled in, for the UI.
async fn decorated(store: &superscience_store::Store) -> Vec<ModelProfile> {
    let profiles = ensure(store).await;
    let id = active_id(store, &profiles).await;
    let vision = vision_id(store, &profiles).await;
    let image_generation = image_generation_id(store, &profiles).await;
    profiles
        .into_iter()
        .map(|mut p| {
            p.has_api_key = !key_for(&p).is_empty();
            p.active = p.id == id;
            p.use_for_vision = vision.as_deref() == Some(p.id.as_str());
            p.use_for_image_generation = image_generation.as_deref() == Some(p.id.as_str());
            p
        })
        .collect()
}

pub(crate) async fn delegation_profiles(store: &superscience_store::Store) -> Vec<ModelProfile> {
    decorated(store)
        .await
        .into_iter()
        .filter(is_chat_model)
        .collect()
}

/// A short unique id derived from the label (or a counter) that isn't taken.
fn fresh_id(existing: &[ModelProfile]) -> String {
    for n in 1..10_000 {
        let id = format!("m{n}");
        if !existing.iter().any(|p| p.id == id) {
            return id;
        }
    }
    "m".into()
}

#[tauri::command]
pub async fn list_models(state: State<'_, crate::AppState>) -> Result<Vec<ModelProfile>, String> {
    Ok(decorated(&state.store).await)
}

fn decorated_providers(
    providers: Vec<ModelProvider>,
    profiles: &[ModelProfile],
) -> Vec<ModelProvider> {
    providers
        .into_iter()
        .map(|mut p| {
            let provider_key = !secret_get(&provider_secret_name(&p.id)).is_empty();
            p.has_api_key = provider_key
                || profiles
                    .iter()
                    .any(|m| m.provider_id == p.id && !key_for(m).is_empty());
            p.model_count = profiles.iter().filter(|m| m.provider_id == p.id).count();
            p
        })
        .collect()
}

#[tauri::command]
pub async fn list_model_providers(
    state: State<'_, crate::AppState>,
) -> Result<Vec<ModelProvider>, String> {
    let profiles = ensure(&state.store).await;
    let providers = ensure_providers(&state.store).await;
    Ok(decorated_providers(providers, &profiles))
}

#[tauri::command]
pub async fn save_model_provider(
    state: State<'_, crate::AppState>,
    mut provider: ModelProvider,
    key: Option<String>,
    clear_key: Option<bool>,
) -> Result<Vec<ModelProvider>, String> {
    let mut providers = ensure_providers(&state.store).await;
    if provider.label.trim().is_empty() {
        return Err("Provider name is required.".into());
    }
    if provider.api_url.trim().is_empty() {
        return Err("API URL is required.".into());
    }
    let protocol = provider.protocol.trim();
    if !matches!(
        protocol,
        "openai" | "openai_responses" | "openai-responses" | "responses" | "anthropic"
    ) {
        return Err("Unsupported provider protocol.".into());
    }
    provider.protocol = match protocol {
        "openai-responses" | "responses" => "openai_responses".into(),
        "anthropic" => "anthropic".into(),
        _ => "openai".into(),
    };
    if provider.id.trim().is_empty() {
        provider.id = fresh_provider_id(&providers);
        provider.builtin = false;
        provider.sort_order = providers.iter().map(|p| p.sort_order).max().unwrap_or(0) + 1;
    } else if let Some(existing) = providers.iter().find(|p| p.id == provider.id) {
        provider.builtin = existing.builtin;
        provider.sort_order = existing.sort_order;
        if existing.builtin {
            provider.id = TCTOKEN_PROVIDER_ID.into();
            // Keep builtin identity; allow URL/label/key edits.
            if provider.label.trim().is_empty() {
                provider.label = TCTOKEN_PROVIDER_LABEL.into();
            }
        }
    }
    let id = provider.id.clone();
    if let Some(existing) = providers.iter_mut().find(|p| p.id == id) {
        *existing = strip_computed_provider(provider);
    } else {
        providers.push(strip_computed_provider(provider));
    }
    save_providers_raw(&state.store, &providers).await?;
    if clear_key == Some(true) {
        let _ = secret_del(&provider_secret_name(&id));
    } else if let Some(k) = key {
        let k = k.trim();
        if !k.is_empty() {
            secret_set(&provider_secret_name(&id), k)?;
        }
    }
    // Keep model mirrors in sync.
    let mut profiles = ensure(&state.store).await;
    let mut dirty = false;
    if let Some(prov) = providers.iter().find(|p| p.id == id) {
        for profile in &mut profiles {
            if profile.provider_id == id
                && (profile.provider != prov.protocol || profile.api_url != prov.api_url)
            {
                profile.provider = prov.protocol.clone();
                profile.api_url = prov.api_url.clone();
                dirty = true;
            }
        }
    }
    if dirty {
        save_raw(&state.store, &profiles).await?;
    }
    crate::clear_idle_agents(&state).await;
    let profiles = ensure(&state.store).await;
    Ok(decorated_providers(providers, &profiles))
}

#[tauri::command]
pub async fn remove_model_provider(
    state: State<'_, crate::AppState>,
    id: String,
) -> Result<Vec<ModelProvider>, String> {
    if id == TCTOKEN_PROVIDER_ID {
        return Err("Built-in providers cannot be removed.".into());
    }
    let profiles = ensure(&state.store).await;
    if profiles.iter().any(|p| p.provider_id == id) {
        return Err("Remove or move all models under this provider first.".into());
    }
    let mut providers = ensure_providers(&state.store).await;
    if providers.iter().any(|p| p.id == id && p.builtin) {
        return Err("Built-in providers cannot be removed.".into());
    }
    providers.retain(|p| p.id != id);
    save_providers_raw(&state.store, &providers).await?;
    let _ = secret_del(&provider_secret_name(&id));
    Ok(decorated_providers(providers, &profiles))
}

#[tauri::command]
pub async fn get_session_model(
    state: State<'_, crate::AppState>,
    window: tauri::WebviewWindow,
    session_id: String,
) -> Result<String, String> {
    let project = state.active(window.label());
    if state
        .store
        .frame_project_id(&session_id)
        .await
        .map_err(|error| error.to_string())?
        .as_deref()
        != Some(project.id.as_str())
    {
        return Err("Session not found".into());
    }
    // ACP-bound frames run through the agent, not an HTTP model. Return the
    // agent's label under an `acp:` marker so message badges don't fall back
    // to the active HTTP model.
    if let Ok(Some(binding)) = state.store.get_acp_session(&session_id).await {
        let label = crate::acp::profile_label(&state.store, &binding.agent_profile_id)
            .await
            .unwrap_or_else(|| "ACP Agent".into());
        return Ok(format!("acp:{label}"));
    }
    Ok(session_profile_id(&state.store, &session_id).await)
}

#[tauri::command]
pub async fn get_session_reasoning_effort(
    state: State<'_, crate::AppState>,
    window: tauri::WebviewWindow,
    session_id: String,
) -> Result<Option<String>, String> {
    let project = state.active(window.label());
    if state
        .store
        .frame_project_id(&session_id)
        .await
        .map_err(|error| error.to_string())?
        .as_deref()
        != Some(project.id.as_str())
    {
        return Err("Session not found".into());
    }
    state
        .store
        .frame_reasoning_effort(&session_id)
        .await
        .map_err(|error| error.to_string())
}

/// Upsert a profile. An empty `id` creates a new one; a non-empty `key` updates
/// the provider secret (a blank key leaves the stored one untouched).
#[tauri::command]
pub async fn save_model(
    state: State<'_, crate::AppState>,
    mut profile: ModelProfile,
    key: Option<String>,
    use_for_vision: Option<bool>,
    use_for_image_generation: Option<bool>,
) -> Result<Vec<ModelProfile>, String> {
    // Explicit top-level param: the flag nested inside `profile` was observed
    // arriving as false through the webview IPC boundary, losing the
    // assignment on save (#131 follow-up).
    let assign_vision = use_for_vision.unwrap_or(profile.use_for_vision);
    let assign_image_generation =
        use_for_image_generation.unwrap_or(profile.use_for_image_generation);
    profile.use_for_vision = assign_vision;
    profile.use_for_image_generation = assign_image_generation;
    let mut profiles = ensure(&state.store).await;
    let providers = ensure_providers(&state.store).await;
    if profile.model.trim().is_empty() {
        return Err("Model is required.".into());
    }
    if profile.provider_id.trim().is_empty() {
        return Err("Provider is required.".into());
    }
    let Some(prov) = providers
        .iter()
        .find(|p| p.id == profile.provider_id)
        .cloned()
    else {
        return Err("Provider not found.".into());
    };
    // Model form no longer owns URL/protocol — always mirror the provider.
    profile.provider = prov.protocol.clone();
    profile.api_url = prov.api_url.clone();
    if assign_vision && !can_describe_images(&profile) {
        return Err("Image analysis requires an API model marked as vision-capable.".into());
    }
    if assign_image_generation && !can_generate_images(&profile) {
        return Err("Image generation currently supports only OpenAI gpt-image-2.".into());
    }
    clamp_to_catalog(&mut profile);
    if profile.label.trim().is_empty() {
        profile.label = profile.model.clone();
    }
    if profile.id.trim().is_empty() {
        profile.id = fresh_id(&profiles);
    }
    let id = profile.id.clone();
    let is_new = !profiles.iter().any(|p| p.id == id);
    // Providers may exist with zero chat models (builtin empty card).
    if let Some(existing) = profiles.iter_mut().find(|p| p.id == id) {
        *existing = profile;
    } else {
        profiles.push(profile);
    }
    save_raw(&state.store, &profiles).await?;
    if assign_vision {
        let _ = state.store.set_setting(VISION_KEY, &id).await;
    } else {
        let cur = state
            .store
            .get_setting(VISION_KEY)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        if cur == id
            && !profiles
                .iter()
                .any(|p| can_describe_images(p) && p.id != id)
        {
            let _ = state.store.set_setting(VISION_KEY, "").await;
        }
    }
    if assign_image_generation {
        let _ = state.store.set_setting(IMAGE_GENERATION_KEY, &id).await;
    } else {
        let current = state
            .store
            .get_setting(IMAGE_GENERATION_KEY)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        if current == id {
            let _ = state.store.set_setting(IMAGE_GENERATION_KEY, "").await;
        }
    }
    if let Some(k) = key {
        let k = k.trim();
        if !k.is_empty() {
            secret_set(&provider_secret_name(&prov.id), k)?;
        }
    }
    // Land the user on a freshly added model so they can edit/use it right away.
    if is_new && profiles.iter().any(|p| p.id == id && is_chat_model(p)) {
        let _ = state.store.set_setting(ACTIVE_KEY, &id).await;
    } else if !profiles.iter().any(|p| p.id == id && is_chat_model(p)) {
        let active = state
            .store
            .get_setting(ACTIVE_KEY)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        if active == id {
            if let Some(first) = profiles.iter().find(|p| is_chat_model(p)) {
                let _ = state.store.set_setting(ACTIVE_KEY, &first.id).await;
            }
        }
    }
    crate::clear_idle_agents(&state).await;
    Ok(decorated(&state.store).await)
}

#[tauri::command]
pub async fn remove_model(
    state: State<'_, crate::AppState>,
    id: String,
) -> Result<Vec<ModelProfile>, String> {
    let mut profiles = ensure(&state.store).await;
    profiles.retain(|p| p.id != id);
    save_raw(&state.store, &profiles).await?;
    let _ = secret_del(&secret_name(&id));
    // If we removed the active profile, fall back to the first remaining one.
    let cur = state
        .store
        .get_setting(ACTIVE_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    if cur == id {
        if let Some(first) = profiles.iter().find(|p| is_chat_model(p)) {
            let _ = state.store.set_setting(ACTIVE_KEY, &first.id).await;
        }
    }
    let image_generation = state
        .store
        .get_setting(IMAGE_GENERATION_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    if image_generation == id {
        let _ = state.store.set_setting(IMAGE_GENERATION_KEY, "").await;
    }
    crate::clear_idle_agents(&state).await;
    Ok(decorated(&state.store).await)
}

/// Reorder `profiles` to match `ids`. Profiles missing from `ids` keep their
/// existing relative order at the end, so a stale client list can never drop a
/// model — it just falls through unmoved. sort_by_key is stable, which is what
/// makes the usize::MAX tail preserve order.
fn reordered(mut profiles: Vec<ModelProfile>, ids: &[String]) -> Vec<ModelProfile> {
    profiles.sort_by_key(|p| ids.iter().position(|id| id == &p.id).unwrap_or(usize::MAX));
    profiles
}

#[tauri::command]
pub async fn reorder_models(
    state: State<'_, crate::AppState>,
    ids: Vec<String>,
) -> Result<Vec<ModelProfile>, String> {
    let profiles = reordered(ensure(&state.store).await, &ids);
    save_raw(&state.store, &profiles).await?;
    Ok(decorated(&state.store).await)
}

#[tauri::command]
pub async fn set_active_model(
    state: State<'_, crate::AppState>,
    _window: tauri::WebviewWindow,
    id: String,
    session_id: Option<String>,
) -> Result<Vec<ModelProfile>, String> {
    let profiles = ensure(&state.store).await;
    if !profiles.iter().any(|p| p.id == id) {
        return Err("Unknown model.".into());
    }
    if profiles
        .iter()
        .find(|p| p.id == id)
        .is_some_and(|p| !is_chat_model(p))
    {
        return Err("Image generation models cannot be used for chat.".into());
    }
    if let Some(session_id) = session_id.filter(|value| !value.is_empty()) {
        let (project, scope) =
            crate::exploration_commands::working_project_for_frame(&state, &session_id).await?;
        let _activity = state.begin_project_activity(&project.id)?;
        let _project_write_locked = crate::exploration_commands::conversation_project_write_locked(
            &state.store,
            &scope,
            Some(&session_id),
        )
        .await?;
        state
            .store
            .set_frame_model(&session_id, &project.id, &id)
            .await
            .map_err(|error| error.to_string())?;
        crate::clear_session_agent(&state, &session_id).await;
    } else {
        state
            .store
            .set_setting(ACTIVE_KEY, &id)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(decorated(&state.store).await)
}

#[tauri::command]
pub async fn set_session_reasoning_effort(
    state: State<'_, crate::AppState>,
    effort: String,
    session_id: String,
) -> Result<(), String> {
    let effort = effort.trim();
    if !effort.is_empty()
        && !matches!(
            effort,
            "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
        )
    {
        return Err("Unknown reasoning effort.".into());
    }
    let (project, scope) =
        crate::exploration_commands::working_project_for_frame(&state, &session_id).await?;
    let _activity = state.begin_project_activity(&project.id)?;
    let _project_write_locked = crate::exploration_commands::conversation_project_write_locked(
        &state.store,
        &scope,
        Some(&session_id),
    )
    .await?;
    // Empty effort clears the override: the session inherits the bound model
    // profile again instead of pinning "provider default" forever.
    let override_value = if effort.is_empty() {
        None
    } else {
        Some(effort)
    };
    state
        .store
        .set_frame_reasoning_effort(&session_id, &project.id, override_value)
        .await
        .map_err(|error| error.to_string())?;
    crate::clear_session_agent(&state, &session_id).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_profile(id: &str, label: &str, model: &str) -> ModelProfile {
        ModelProfile {
            id: id.into(),
            label: label.into(),
            provider: "openai".into(),
            api_url: "u".into(),
            provider_id: TCTOKEN_PROVIDER_ID.into(),
            model: model.into(),
            has_api_key: false,
            active: false,
            max_tokens: 0,
            context_window: DEFAULT_CONTEXT_WINDOW,
            reasoning_effort: String::new(),
            supports_vision: false,
            use_for_vision: false,
            use_for_image_generation: false,
        }
    }

    #[tokio::test]
    async fn save_then_reload_keeps_vision_assignment() {
        // repro for "checkbox lost after save+reopen": full backend round-trip
        // through save_raw + VISION_KEY + decorated.
        let tmp = std::env::temp_dir().join(format!(
            "superscience_vision_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = superscience_store::Store::open(&tmp).await.unwrap();
        let mut p = test_profile("m1", "claude", "claude-opus-4-8");
        p.supports_vision = true;
        save_raw(&store, &[test_profile("m0", "text", "deepseek"), p])
            .await
            .unwrap();
        store.set_setting(VISION_KEY, "m1").await.unwrap();
        let out = decorated(&store).await;
        let m1 = out.iter().find(|p| p.id == "m1").unwrap();
        assert!(m1.supports_vision, "capability lost in persistence");
        assert!(m1.use_for_vision, "vision assignment lost after reload");
        assert!(!out.iter().find(|p| p.id == "m0").unwrap().use_for_vision);
        let json = serde_json::to_value(out).unwrap();
        assert_eq!(
            json[1]["use_for_vision"], true,
            "IPC response lost vision assignment"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn reordered_moves_named_and_keeps_unlisted_at_end() {
        let ids = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let src = vec![
            test_profile("a", "a", "a"),
            test_profile("b", "b", "b"),
            test_profile("c", "c", "c"),
        ];
        // Full reversal.
        let out = reordered(src.clone(), &ids(&["c", "b", "a"]));
        assert_eq!(
            out.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["c", "b", "a"]
        );
        // Only "c" named: it leads, unlisted a/b keep their original order.
        let out = reordered(src.clone(), &ids(&["c"]));
        assert_eq!(
            out.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["c", "a", "b"]
        );
        // Unknown id in the list is ignored, real ids still reorder.
        let out = reordered(src, &ids(&["ghost", "b", "a"]));
        assert_eq!(
            out.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["b", "a", "c"]
        );
    }

    #[tokio::test]
    async fn session_profile_binding_does_not_change_other_sessions() {
        let tmp = std::env::temp_dir().join(format!(
            "superscience_session_models_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = superscience_store::Store::open(&tmp).await.unwrap();
        store.create_project("p", "project", "").await.unwrap();
        save_raw(
            &store,
            &[
                test_profile("m1", "first", "model-1"),
                test_profile("m2", "second", "model-2"),
            ],
        )
        .await
        .unwrap();
        store.set_setting(ACTIVE_KEY, "m1").await.unwrap();
        store
            .create_frame("a", "p", "SUPERSCIENCE", "m1")
            .await
            .unwrap();
        store
            .create_frame("b", "p", "SUPERSCIENCE", "m1")
            .await
            .unwrap();

        store.set_frame_model("a", "p", "m2").await.unwrap();

        assert_eq!(session_profile_id(&store, "a").await, "m2");
        assert_eq!(session_profile_id(&store, "b").await, "m1");
        drop(store);
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn session_reasoning_override_does_not_change_profile_or_sibling() {
        let tmp = std::env::temp_dir().join(format!(
            "wisp_session_reasoning_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = superscience_store::Store::open(&tmp).await.unwrap();
        store.create_project("p", "project", "").await.unwrap();
        let mut profile = test_profile("m1", "reasoner", "model-1");
        profile.reasoning_effort = "max".into();
        save_raw(&store, &[profile]).await.unwrap();
        store.set_setting(ACTIVE_KEY, "m1").await.unwrap();
        store.create_frame("a", "p", "OPERON", "m1").await.unwrap();
        store.create_frame("b", "p", "OPERON", "m1").await.unwrap();

        store
            .set_frame_reasoning_effort("a", "p", Some("high"))
            .await
            .unwrap();

        assert_eq!(session_reasoning_effort(&store, "a", "max").await, "high");
        assert_eq!(session_reasoning_effort(&store, "b", "max").await, "max");
        assert_eq!(profile_llm(&store, "m1").await.unwrap().5, "max");
        drop(store);
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn cleared_or_empty_session_override_inherits_profile() {
        let tmp = std::env::temp_dir().join(format!(
            "wisp_session_reasoning_clear_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = superscience_store::Store::open(&tmp).await.unwrap();
        store.create_project("p", "project", "").await.unwrap();
        store.create_frame("a", "p", "OPERON", "m1").await.unwrap();
        store.create_frame("b", "p", "OPERON", "m1").await.unwrap();

        store
            .set_frame_reasoning_effort("a", "p", Some("high"))
            .await
            .unwrap();
        assert_eq!(session_reasoning_effort(&store, "a", "max").await, "high");
        // Clearing the override (selecting "default" in the composer) makes
        // the session follow the profile default again.
        store
            .set_frame_reasoning_effort("a", "p", None)
            .await
            .unwrap();
        assert_eq!(session_reasoning_effort(&store, "a", "max").await, "max");
        // Legacy rows hold Some("") for "provider default"; they must not pin
        // the session away from the profile either.
        store
            .set_frame_reasoning_effort("b", "p", Some(""))
            .await
            .unwrap();
        assert_eq!(session_reasoning_effort(&store, "b", "max").await, "max");
        drop(store);
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn vision_capability_follows_the_input_profile() {
        let tmp = std::env::temp_dir().join(format!(
            "superscience_input_vision_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = superscience_store::Store::open(&tmp).await.unwrap();
        let text = test_profile("m0", "text", "text-model");
        let mut vision = test_profile("m1", "vision", "vision-model");
        vision.supports_vision = true;
        save_raw(&store, &[text, vision]).await.unwrap();
        store.set_setting(ACTIVE_KEY, "m0").await.unwrap();

        assert!(!supports_vision(&store, None).await);
        assert!(supports_vision(&store, Some("m1")).await);
        assert!(!supports_vision(&store, Some("missing")).await);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn use_for_vision_survives_deserialization() {
        // Repro for the "checkbox lost after save" report: the incoming
        // command payload must keep both role assignments.
        let p: ModelProfile = serde_json::from_str(
            r#"{"id":"m1","label":"l","provider":"anthropic","api_url":"u","model":"m",
                "max_tokens":8192,"reasoning_effort":"medium",
                "supports_vision":true,"use_for_vision":true,
                "use_for_image_generation":true}"#,
        )
        .unwrap();
        assert!(p.supports_vision);
        assert!(p.use_for_vision, "use_for_vision dropped on deserialize");
        assert!(
            p.use_for_image_generation,
            "use_for_image_generation dropped on deserialize"
        );
        assert_eq!(p.context_window, DEFAULT_CONTEXT_WINDOW);
    }

    #[test]
    fn context_window_survives_profile_roundtrip() {
        let mut profile = test_profile("m1", "reader", "cheap-reader");
        profile.context_window = 32_768;
        let json = serde_json::to_string(&profile).unwrap();
        let restored: ModelProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.context_window, 32_768);
    }

    fn kimi_coding_profile(model: &str) -> ModelProfile {
        let mut profile = test_profile("m1", "kimi", model);
        profile.api_url = "https://api.kimi.com/coding/v1".into();
        profile
    }

    #[test]
    fn clamp_to_catalog_caps_over_declared_limits() {
        let mut profile = kimi_coding_profile("k3-256k");
        profile.context_window = 1_000_000;
        profile.max_tokens = 999_999;
        clamp_to_catalog(&mut profile);
        assert_eq!(profile.context_window, 262_144);
        assert_eq!(profile.max_tokens, 131_072);
    }

    #[test]
    fn clamp_to_catalog_keeps_unset_and_unknown_values() {
        // max_tokens = 0 means "unset" and stays.
        let mut profile = kimi_coding_profile("k3-256k");
        profile.max_tokens = 0;
        clamp_to_catalog(&mut profile);
        assert_eq!(profile.max_tokens, 0);
        // Unknown models are left alone.
        let mut unknown = test_profile("m2", "x", "totally-unknown");
        unknown.context_window = 500_000;
        unknown.max_tokens = 90_000;
        clamp_to_catalog(&mut unknown);
        assert_eq!(unknown.context_window, 500_000);
        assert_eq!(unknown.max_tokens, 90_000);
    }

    #[test]
    fn effective_context_window_respects_catalog_ceiling() {
        let mut over = kimi_coding_profile("k3-256k");
        over.context_window = 1_000_000;
        assert_eq!(effective_context_window(&over), 262_144);
        // Unknown models keep their declared value.
        let mut unknown = test_profile("m2", "x", "totally-unknown");
        unknown.context_window = 500_000;
        assert_eq!(effective_context_window(&unknown), 500_000);
        // Degenerate values still fall back to the default window.
        unknown.context_window = 100;
        assert_eq!(effective_context_window(&unknown), DEFAULT_CONTEXT_WINDOW);
    }

    #[test]
    fn fresh_id_skips_taken() {
        let existing = vec![test_profile("m1", "a", "x"), test_profile("m2", "b", "y")];
        assert_eq!(fresh_id(&existing), "m3");
        assert_eq!(fresh_id(&[]), "m1");
    }

    #[test]
    fn vision_assignment_marker_is_serialized_for_ui() {
        let mut profile = test_profile("m1", "vision", "v");
        profile.supports_vision = true;
        profile.use_for_vision = true;
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("supports_vision"));
        assert!(json.contains("\"use_for_vision\":true"));
    }

    #[test]
    fn vision_capability_uses_marker() {
        let mut profile = test_profile("m1", "vision", "v");
        profile.supports_vision = true;
        assert!(can_describe_images(&profile));
        profile.supports_vision = false;
        assert!(!can_describe_images(&profile));
        profile.supports_vision = true;
        profile.model = "gpt-image-2".into();
        assert!(!can_describe_images(&profile));
    }

    #[test]
    fn image_generation_accepts_only_openai_gpt_image_2() {
        let mut profile = test_profile("image", "image", "gpt-image-2");
        profile.provider = "openai_responses".into();
        assert!(can_generate_images(&profile));

        profile.provider = "anthropic".into();
        assert!(!can_generate_images(&profile));
        profile.provider = "openai".into();
        profile.model = "gpt-image-1".into();
        assert!(!can_generate_images(&profile));
    }

    #[tokio::test]
    async fn image_generation_requires_an_explicit_gpt_image_2_assignment() {
        let tmp = std::env::temp_dir().join(format!(
            "superscience_image_gen_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = superscience_store::Store::open(&tmp).await.unwrap();
        let chat = test_profile("chat", "chat", "gpt-5.5");
        let mut image = test_profile("image", "image", "gpt-image-2");
        image.provider = "openai_responses".into();
        save_raw(&store, &[chat, image]).await.unwrap();

        assert!(image_generation_config(&store).await.is_none());
        store
            .set_setting(IMAGE_GENERATION_KEY, "image")
            .await
            .unwrap();
        let (url, model, _key) = image_generation_config(&store).await.unwrap();
        assert_eq!(url, "u");
        assert_eq!(model, "gpt-image-2");

        let decorated = decorated(&store).await;
        assert!(
            decorated
                .iter()
                .find(|profile| profile.id == "image")
                .unwrap()
                .use_for_image_generation
        );
        assert_eq!(
            delegation_profiles(&store)
                .await
                .iter()
                .map(|profile| profile.id.as_str())
                .collect::<Vec<_>>(),
            ["chat"]
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn image_generation_profile_is_never_the_active_chat_model() {
        let tmp = std::env::temp_dir().join(format!(
            "superscience_image_gen_active_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = superscience_store::Store::open(&tmp).await.unwrap();
        let chat = test_profile("chat", "chat", "gpt-5.5");
        let image = test_profile("image", "image", "gpt-image-2");
        save_raw(&store, &[chat, image]).await.unwrap();
        store.set_setting(ACTIVE_KEY, "image").await.unwrap();

        assert_eq!(active_profile_id(&store).await, "chat");
        assert_eq!(profile_context_window(&store, "image").await, None);
        let decorated = decorated(&store).await;
        assert!(
            decorated
                .iter()
                .find(|profile| profile.id == "chat")
                .unwrap()
                .active
        );
        assert!(
            !decorated
                .iter()
                .find(|profile| profile.id == "image")
                .unwrap()
                .active
        );
        let _ = std::fs::remove_file(&tmp);
    }

    // The write-through cache must stay coherent: a set is readable without a
    // fresh secrets-file read, and a delete reads back as absent (not the old value).
    #[test]
    fn secret_cache_write_through() {
        let name = "model_key:__cache_coherence_test__";
        secret_set(name, "sk-abc").unwrap();
        assert_eq!(secret_get(name), "sk-abc");
        secret_del(name).unwrap();
        assert_eq!(secret_get(name), "");
    }

    // Storing a credential surfaces it in service_env under its env var;
    // clearing removes it; an unknown id is rejected.
    #[test]
    fn credential_registry_roundtrip() {
        store_credential("ncbi_email", "me@lab.org").unwrap();
        assert!(credential_status()
            .iter()
            .any(|(id, ok)| id == "ncbi_email" && *ok));
        assert!(service_env()
            .iter()
            .any(|(k, v)| k == "NCBI_EMAIL" && v == "me@lab.org"));

        store_credential("infinisynapse_api_key", "sk-infini").unwrap();
        assert!(service_env()
            .iter()
            .any(|(k, v)| k == "INFINISYNAPSE_API_KEY" && v == "sk-infini"));
        store_credential("infinisynapse_api_key", "").unwrap();

        store_credential("scimaster_api_key", "sk-sci").unwrap();
        assert!(service_env()
            .iter()
            .any(|(k, v)| k == "SCIMASTER_API_KEY" && v == "sk-sci"));
        store_credential("scimaster_api_key", "").unwrap();

        store_credential("ncbi_email", "  ").unwrap(); // blank clears
        assert!(!service_env().iter().any(|(k, _)| k == "NCBI_EMAIL"));

        assert!(store_credential("nonexistent", "x").is_err());
    }

    #[tokio::test]
    async fn custom_credentials_keep_values_out_of_sqlite_and_join_service_env() {
        let tmp = std::env::temp_dir().join(format!(
            "superscience_custom_credentials_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = superscience_store::Store::open(&tmp).await.unwrap();
        let suffix = uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .to_ascii_uppercase();
        let env_var = format!("SUPERSCIENCE_CUSTOM_TEST_{suffix}");
        let secret = format!("custom-secret-{suffix}");

        assert!(
            add_custom_credential(&store, "MetaSo", "BAD-NAME", "secret")
                .await
                .unwrap_err()
                .contains("Environment variable")
        );
        assert!(
            add_custom_credential(&store, "Duplicate", "OPENALEX_API_KEY", "secret")
                .await
                .unwrap_err()
                .contains("already uses")
        );

        let saved = add_custom_credential(&store, "MetaSo", &env_var, &secret)
            .await
            .unwrap();
        assert!(saved.present);
        assert_eq!(saved.env_var, env_var);
        assert!(custom_credential_status(&store)
            .await
            .unwrap()
            .iter()
            .any(|credential| credential.id == saved.id && credential.present));
        assert!(service_env()
            .iter()
            .any(|(name, value)| name == &env_var && value == &secret));

        let raw = store
            .get_setting(CUSTOM_CREDENTIALS_KEY)
            .await
            .unwrap()
            .unwrap();
        assert!(raw.contains("MetaSo"));
        assert!(raw.contains(&env_var));
        assert!(!raw.contains(&secret));

        store_credential(&saved.id, "replacement").unwrap();
        assert!(service_env()
            .iter()
            .any(|(name, value)| name == &env_var && value == "replacement"));

        // Re-adding the same env var upserts the existing row instead of
        // erroring, even after its value was cleared (#335).
        store_credential(&saved.id, "").unwrap();
        let updated = add_custom_credential(&store, "MetaSo v2", &env_var, "second")
            .await
            .unwrap();
        assert_eq!(updated.id, saved.id);
        assert_eq!(updated.name, "MetaSo v2");
        assert!(updated.present);
        assert!(service_env()
            .iter()
            .any(|(name, value)| name == &env_var && value == "second"));
        assert_eq!(custom_credential_status(&store).await.unwrap().len(), 1);

        remove_custom_credential(&store, &saved.id).await.unwrap();
        assert!(custom_credential_status(&store).await.unwrap().is_empty());
        assert!(!service_env().iter().any(|(name, _)| name == &env_var));
        let _ = std::fs::remove_file(tmp);
    }

    #[tokio::test]
    async fn profile_key_reads_the_requested_profile() {
        let tmp = std::env::temp_dir().join(format!(
            "superscience_profile_key_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = superscience_store::Store::open(&tmp).await.unwrap();
        // Distinct URLs so migration creates separate providers (keys are
        // provider-scoped after the cards redesign).
        let mut deepseek = test_profile("default", "deepseek", "deepseek-v4-pro");
        deepseek.api_url = "https://api.deepseek.com/v1".into();
        deepseek.provider_id.clear();
        let mut glm = test_profile("glm", "glm", "glm-5.2");
        glm.api_url = "https://open.bigmodel.cn/api/paas/v4".into();
        glm.provider_id.clear();
        save_raw(&store, &[deepseek, glm]).await.unwrap();
        secret_set(&secret_name("default"), "sk-default").unwrap();
        secret_set(&secret_name("glm"), "sk-glm").unwrap();

        assert_eq!(profile_key(&store, "glm").await.as_deref(), Some("sk-glm"));
        assert_eq!(
            profile_key(&store, "default").await.as_deref(),
            Some("sk-default")
        );
        assert_eq!(profile_key(&store, "missing").await, None);

        let _ = secret_del(&secret_name("default"));
        let _ = secret_del(&secret_name("glm"));
        let profiles = ensure(&store).await;
        for p in &profiles {
            let _ = secret_del(&provider_secret_name(&p.provider_id));
        }
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn migrate_clusters_same_url_and_shares_provider_key() {
        let tmp = std::env::temp_dir().join(format!(
            "superscience_provider_migrate_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = superscience_store::Store::open(&tmp).await.unwrap();
        let mut a = test_profile("m1", "A", "model-a");
        a.api_url = "https://example.com/v1".into();
        a.provider_id.clear();
        let mut b = test_profile("m2", "B", "model-b");
        b.api_url = "https://example.com/v1/".into(); // trailing slash → same cluster
        b.provider_id.clear();
        save_raw(&store, &[a, b]).await.unwrap();
        secret_set(&secret_name("m1"), "sk-shared").unwrap();

        let profiles = ensure(&store).await;
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].provider_id, profiles[1].provider_id);
        assert_ne!(profiles[0].provider_id, TCTOKEN_PROVIDER_ID);
        assert_eq!(key_for(&profiles[0]), "sk-shared");
        assert_eq!(key_for(&profiles[1]), "sk-shared");

        let providers = load_providers_raw(&store).await;
        assert!(providers.iter().any(|p| p.id == TCTOKEN_PROVIDER_ID && p.builtin));
        assert_eq!(providers[0].id, TCTOKEN_PROVIDER_ID);

        let _ = secret_del(&secret_name("m1"));
        let _ = secret_del(&provider_secret_name(&profiles[0].provider_id));
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn empty_install_seeds_builtin_tctoken_only() {
        let tmp = std::env::temp_dir().join(format!(
            "superscience_provider_empty_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = superscience_store::Store::open(&tmp).await.unwrap();
        let profiles = ensure(&store).await;
        assert!(profiles.is_empty());
        let providers = load_providers_raw(&store).await;
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, TCTOKEN_PROVIDER_ID);
        assert_eq!(providers[0].api_url, TCTOKEN_PROVIDER_URL);
        assert!(providers[0].builtin);
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn builtin_provider_cannot_be_removed() {
        let tmp = std::env::temp_dir().join(format!(
            "superscience_provider_builtin_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = superscience_store::Store::open(&tmp).await.unwrap();
        let _ = ensure(&store).await;
        // Simulate remove_model_provider guard without Tauri State.
        let id = TCTOKEN_PROVIDER_ID.to_string();
        assert!(id == TCTOKEN_PROVIDER_ID);
        let providers = load_providers_raw(&store).await;
        assert!(providers.iter().any(|p| p.id == id && p.builtin));
        let _ = std::fs::remove_file(&tmp);
    }
}
