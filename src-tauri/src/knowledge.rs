//! Knowledge-base settings and WeKnora REST adapter.
//!
//! Secrets stay in the OS/file secret store (`weknora_api_key`), never SQLite
//! and never `CREDENTIALS` / `service_env`.

use super::{clear_idle_agents, llm_proxy, AppState};
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;
use superscience_dto::{
    KnowledgeBaseSummary, KnowledgeConnectionTest, KnowledgeSettings, WeKnoraSettings,
};
use superscience_store::secrets::Secret;
use superscience_store::Store;
use tauri::State;

const PROVIDER_WEKNORA: &str = "weknora";
const SETTING_PROVIDER: &str = "knowledge_provider";
const SETTING_BASE_URL: &str = "weknora_base_url";
const SETTING_KB_IDS: &str = "weknora_knowledge_base_ids";
const SETTING_MATCH_COUNT: &str = "weknora_match_count";
const SECRET_API_KEY: &str = "weknora_api_key";
const DEFAULT_BASE_URL: &str = "http://localhost:8080/api/v1";
const DEFAULT_MATCH_COUNT: u32 = 8;
const MAX_MATCH_COUNT: u32 = 32;
const HTTP_TIMEOUT_SECS: u64 = 30;

#[derive(Clone, Debug)]
pub(crate) struct WeKnoraRuntime {
    pub base_url: String,
    pub api_key: String,
    pub knowledge_base_ids: Vec<String>,
    pub match_count: u32,
}

#[derive(Clone, Debug)]
pub(crate) enum KnowledgeRuntime {
    WeKnora(WeKnoraRuntime),
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct KnowledgeHit {
    pub content: String,
    pub score: Option<f64>,
    pub source_id: String,
    pub title: String,
    pub filename: String,
    pub provider: String,
}

pub(crate) fn normalize_weknora_base_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return DEFAULT_BASE_URL.into();
    }
    let without_slash = trimmed.trim_end_matches('/');
    match url::Url::parse(without_slash) {
        Ok(parsed) => {
            let path = parsed.path();
            if path.is_empty() || path == "/" {
                format!("{without_slash}/api/v1")
            } else {
                without_slash.to_string()
            }
        }
        Err(_) => without_slash.to_string(),
    }
}

pub(crate) fn parse_knowledge_base_ids(raw: &str) -> Vec<String> {
    raw.split(|c: char| c == ',' || c.is_whitespace())
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

pub(crate) fn clamp_match_count(value: u32) -> u32 {
    value.clamp(1, MAX_MATCH_COUNT)
}

pub(crate) fn parse_provider(raw: &str) -> Result<Option<&'static str>, String> {
    match raw.trim() {
        "" => Ok(None),
        PROVIDER_WEKNORA => Ok(Some(PROVIDER_WEKNORA)),
        other => Err(format!(
            "Unknown knowledge provider '{other}'. First release only supports weknora."
        )),
    }
}

pub(crate) fn build_search_body(query: &str, ids: &[String]) -> Result<Value, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("knowledge_search needs a non-empty query.".into());
    }
    if ids.is_empty() {
        return Err(missing_kb_ids_message());
    }
    if ids.len() == 1 {
        return Ok(json!({
            "query": query,
            "knowledge_base_id": ids[0],
        }));
    }
    Ok(json!({
        "query": query,
        "knowledge_base_ids": ids,
    }))
}

pub(crate) fn missing_kb_ids_message() -> String {
    "knowledge_search needs a knowledge base ID. Set a default in Settings → Knowledge Base, or pass knowledge_base_id / knowledge_base_ids.".into()
}

pub(crate) fn map_http_error(status: u16, body: &str) -> String {
    let snippet = body.trim();
    let snippet = if snippet.len() > 400 {
        format!("{}…", &snippet[..400])
    } else {
        snippet.to_string()
    };
    match status {
        401 | 403 => format!(
            "WeKnora rejected the API key ({status}). Open Settings → Knowledge Base and update the key.{suffix}",
            suffix = if snippet.is_empty() {
                String::new()
            } else {
                format!(" {snippet}")
            }
        ),
        _ => {
            if snippet.is_empty() {
                format!("WeKnora request failed with HTTP {status}.")
            } else {
                format!("WeKnora request failed with HTTP {status}: {snippet}")
            }
        }
    }
}

fn data_array(body: &Value) -> Option<&Vec<Value>> {
    if let Some(rows) = body.get("data").and_then(Value::as_array) {
        return Some(rows);
    }
    let data = body.get("data")?;
    data.get("list")
        .or_else(|| data.get("items"))
        .or_else(|| data.get("knowledge_bases"))
        .and_then(Value::as_array)
}

pub(crate) fn parse_search_hits(
    body: &Value,
    match_count: u32,
    provider: &str,
) -> Vec<KnowledgeHit> {
    let Some(rows) = data_array(body) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let content = row
                .get("content")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())?
                .to_string();
            let source_id = row
                .get("knowledge_id")
                .or_else(|| row.get("id"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let title = row
                .get("knowledge_title")
                .or_else(|| row.get("title"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let filename = row
                .get("knowledge_filename")
                .or_else(|| row.get("filename"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let score = row.get("score").and_then(Value::as_f64);
            Some(KnowledgeHit {
                content,
                score,
                source_id,
                title,
                filename,
                provider: provider.to_string(),
            })
        })
        .take(clamp_match_count(match_count) as usize)
        .collect()
}

pub(crate) fn parse_knowledge_bases(body: &Value) -> Vec<KnowledgeBaseSummary> {
    let Some(rows) = data_array(body) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let id = row
                .get("id")
                .or_else(|| row.get("ID"))
                .or_else(|| row.get("knowledge_base_id"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())?
                .to_string();
            let name = row
                .get("name")
                .or_else(|| row.get("Name"))
                .or_else(|| row.get("title"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            Some(KnowledgeBaseSummary { id, name })
        })
        .collect()
}

fn http_client(proxy: Option<&str>) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent("superscience")
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS));
    match proxy.map(str::trim) {
        None | Some("") => {}
        Some("none") => builder = builder.no_proxy(),
        Some(proxy) => {
            builder = builder.proxy(
                reqwest::Proxy::all(proxy)
                    .map_err(|error| format!("invalid knowledge-base proxy: {error}"))?,
            );
        }
    }
    builder
        .build()
        .map_err(|error| format!("failed to build HTTP client: {error}"))
}

async fn weknora_request(
    method: reqwest::Method,
    url: &str,
    api_key: &str,
    body: Option<&Value>,
    proxy: Option<&str>,
) -> Result<(u16, Value), String> {
    let client = http_client(proxy)?;
    let mut request = client
        .request(method, url)
        .header("X-API-Key", api_key)
        .header("Accept", "application/json");
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("WeKnora request failed: {error}"))?;
    let status = response.status().as_u16();
    let text = response
        .text()
        .await
        .map_err(|error| format!("WeKnora response could not be read: {error}"))?;
    if !(200..300).contains(&status) {
        return Err(map_http_error(status, &text));
    }
    if text.trim().is_empty() {
        return Ok((status, json!({})));
    }
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("WeKnora returned non-JSON: {error}"))?;
    Ok((status, value))
}

pub(crate) async fn weknora_search(
    runtime: &WeKnoraRuntime,
    query: &str,
    override_ids: Option<Vec<String>>,
    proxy: Option<&str>,
) -> Result<Vec<KnowledgeHit>, String> {
    let ids = override_ids
        .filter(|ids| !ids.is_empty())
        .unwrap_or_else(|| runtime.knowledge_base_ids.clone());
    let body = build_search_body(query, &ids)?;
    let url = format!(
        "{}/knowledge-search",
        runtime.base_url.trim_end_matches('/')
    );
    let (_status, value) = weknora_request(
        reqwest::Method::POST,
        &url,
        &runtime.api_key,
        Some(&body),
        proxy,
    )
    .await?;
    Ok(parse_search_hits(
        &value,
        runtime.match_count,
        PROVIDER_WEKNORA,
    ))
}

pub(crate) async fn weknora_list_bases(
    base_url: &str,
    api_key: &str,
    proxy: Option<&str>,
) -> Result<Vec<KnowledgeBaseSummary>, String> {
    let url = format!("{}/knowledge-bases", base_url.trim_end_matches('/'));
    let (_status, value) =
        weknora_request(reqwest::Method::GET, &url, api_key, None, proxy).await?;
    Ok(parse_knowledge_bases(&value))
}

fn secret_get(name: &str) -> Result<String, String> {
    Secret::get(name).map_err(|error| error.to_string())
}

fn secret_has(name: &str) -> bool {
    Secret::get(name).is_ok()
}

fn secret_set(name: &str, value: &str) -> Result<(), String> {
    Secret::set(name, value).map_err(|error| error.to_string())
}

pub(crate) async fn load_settings(store: &Store) -> KnowledgeSettings {
    let provider = store
        .get_setting(SETTING_PROVIDER)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let base_url = store
        .get_setting(SETTING_BASE_URL)
        .await
        .ok()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.into());
    let knowledge_base_ids = store
        .get_setting(SETTING_KB_IDS)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let match_count = store
        .get_setting(SETTING_MATCH_COUNT)
        .await
        .ok()
        .flatten()
        .and_then(|value| value.parse().ok())
        .map(clamp_match_count)
        .unwrap_or(DEFAULT_MATCH_COUNT);
    let has_api_key = tokio::task::spawn_blocking(|| secret_has(SECRET_API_KEY))
        .await
        .unwrap_or(false);
    KnowledgeSettings {
        provider,
        weknora: WeKnoraSettings {
            base_url,
            has_api_key,
            api_key: String::new(),
            knowledge_base_ids,
            match_count,
        },
    }
}

pub(crate) async fn load_runtime(store: &Store) -> Result<Option<KnowledgeRuntime>, String> {
    let settings = load_settings(store).await;
    runtime_from_settings(&settings, None).await
}

async fn resolve_api_key(submitted: &str) -> Result<String, String> {
    let submitted = submitted.trim();
    if !submitted.is_empty() {
        return Ok(submitted.to_string());
    }
    tokio::task::spawn_blocking(|| secret_get(SECRET_API_KEY).unwrap_or_default())
        .await
        .map_err(|error| error.to_string())
}

async fn runtime_from_settings(
    settings: &KnowledgeSettings,
    override_key: Option<String>,
) -> Result<Option<KnowledgeRuntime>, String> {
    let Some(provider) = parse_provider(&settings.provider)? else {
        return Ok(None);
    };
    match provider {
        PROVIDER_WEKNORA => {
            let api_key = if let Some(key) = override_key {
                key
            } else {
                resolve_api_key(&settings.weknora.api_key).await?
            };
            Ok(Some(KnowledgeRuntime::WeKnora(WeKnoraRuntime {
                base_url: normalize_weknora_base_url(&settings.weknora.base_url),
                api_key,
                knowledge_base_ids: parse_knowledge_base_ids(&settings.weknora.knowledge_base_ids),
                match_count: clamp_match_count(settings.weknora.match_count),
            })))
        }
        _ => Ok(None),
    }
}

pub(crate) fn runtime_is_ready(runtime: &KnowledgeRuntime) -> bool {
    match runtime {
        KnowledgeRuntime::WeKnora(weknora) => {
            !weknora.base_url.is_empty()
                && !weknora.api_key.trim().is_empty()
                && !weknora.knowledge_base_ids.is_empty()
        }
    }
}

pub(crate) fn missing_config_message(runtime: &KnowledgeRuntime) -> String {
    match runtime {
        KnowledgeRuntime::WeKnora(weknora) => {
            let mut missing = Vec::new();
            if weknora.base_url.trim().is_empty() {
                missing.push("API URL");
            }
            if weknora.api_key.trim().is_empty() {
                missing.push("API key");
            }
            if weknora.knowledge_base_ids.is_empty() {
                missing.push("default knowledge base ID");
            }
            if missing.is_empty() {
                "WeKnora is not configured. Open Settings → Knowledge Base.".into()
            } else {
                format!(
                    "WeKnora is missing {}. Open Settings → Knowledge Base to add {}.",
                    missing.join(", "),
                    if missing.len() == 1 { "it" } else { "them" }
                )
            }
        }
    }
}

#[tauri::command]
pub(super) async fn get_knowledge_settings(
    state: State<'_, AppState>,
) -> Result<KnowledgeSettings, String> {
    Ok(load_settings(&state.store).await)
}

#[tauri::command]
pub(super) async fn set_knowledge_settings(
    state: State<'_, AppState>,
    settings: KnowledgeSettings,
) -> Result<KnowledgeSettings, String> {
    let provider = parse_provider(&settings.provider)?
        .unwrap_or("")
        .to_string();
    let base_url = if settings.weknora.base_url.trim().is_empty() {
        DEFAULT_BASE_URL.to_string()
    } else {
        normalize_weknora_base_url(&settings.weknora.base_url)
    };
    let kb_ids = settings.weknora.knowledge_base_ids.trim().to_string();
    let match_count = clamp_match_count(settings.weknora.match_count);
    state
        .store
        .set_setting(SETTING_PROVIDER, &provider)
        .await
        .map_err(|error| error.to_string())?;
    state
        .store
        .set_setting(SETTING_BASE_URL, &base_url)
        .await
        .map_err(|error| error.to_string())?;
    state
        .store
        .set_setting(SETTING_KB_IDS, &kb_ids)
        .await
        .map_err(|error| error.to_string())?;
    state
        .store
        .set_setting(SETTING_MATCH_COUNT, &match_count.to_string())
        .await
        .map_err(|error| error.to_string())?;
    if !settings.weknora.api_key.trim().is_empty() {
        let key = settings.weknora.api_key.trim().to_string();
        tokio::task::spawn_blocking(move || secret_set(SECRET_API_KEY, &key))
            .await
            .map_err(|error| error.to_string())??;
    }
    clear_idle_agents(&state).await;
    Ok(load_settings(&state.store).await)
}

#[tauri::command]
pub(super) async fn test_knowledge_connection(
    state: State<'_, AppState>,
    settings: Option<KnowledgeSettings>,
) -> Result<KnowledgeConnectionTest, String> {
    let settings = match settings {
        Some(settings) => settings,
        None => load_settings(&state.store).await,
    };
    let Some(runtime) = runtime_from_settings(&settings, None).await? else {
        return Ok(KnowledgeConnectionTest {
            ok: false,
            message: "Choose a knowledge provider in Settings → Knowledge Base.".into(),
            knowledge_bases: Vec::new(),
        });
    };
    match runtime {
        KnowledgeRuntime::WeKnora(weknora) => {
            if weknora.api_key.trim().is_empty() {
                return Ok(KnowledgeConnectionTest {
                    ok: false,
                    message:
                        "WeKnora is missing an API key. Paste it in Settings → Knowledge Base."
                            .into(),
                    knowledge_bases: Vec::new(),
                });
            }
            match weknora_list_bases(&weknora.base_url, &weknora.api_key, llm_proxy().as_deref())
                .await
            {
                Ok(knowledge_bases) => {
                    let count = knowledge_bases.len();
                    Ok(KnowledgeConnectionTest {
                        ok: true,
                        message: if count == 0 {
                            "Connected. No knowledge bases were returned.".into()
                        } else {
                            format!("Connected. Found {count} knowledge base(s).")
                        },
                        knowledge_bases,
                    })
                }
                Err(message) => Ok(KnowledgeConnectionTest {
                    ok: false,
                    message,
                    knowledge_bases: Vec::new(),
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_host_only_url_to_api_v1() {
        assert_eq!(
            normalize_weknora_base_url("http://localhost:8080"),
            "http://localhost:8080/api/v1"
        );
        assert_eq!(
            normalize_weknora_base_url("http://localhost:8080/"),
            "http://localhost:8080/api/v1"
        );
        assert_eq!(
            normalize_weknora_base_url("http://localhost:8080/api/v1/"),
            "http://localhost:8080/api/v1"
        );
        assert_eq!(normalize_weknora_base_url(""), DEFAULT_BASE_URL);
    }

    #[test]
    fn parses_comma_and_whitespace_ids() {
        assert_eq!(
            parse_knowledge_base_ids("kb-1, kb-2\nkb-3"),
            vec!["kb-1", "kb-2", "kb-3"]
        );
        assert!(parse_knowledge_base_ids("  , \n ").is_empty());
    }

    #[test]
    fn search_body_requires_ids_and_uses_single_or_list() {
        assert!(build_search_body("q", &[]).is_err());
        assert_eq!(
            build_search_body("how", &["kb-1".into()]).unwrap(),
            json!({"query": "how", "knowledge_base_id": "kb-1"})
        );
        assert_eq!(
            build_search_body("how", &["kb-1".into(), "kb-2".into()]).unwrap(),
            json!({"query": "how", "knowledge_base_ids": ["kb-1", "kb-2"]})
        );
    }

    #[test]
    fn parse_hits_maps_weknora_fields_and_truncates() {
        let body = json!({
            "success": true,
            "data": [
                {
                    "id": "chunk-1",
                    "content": "first",
                    "knowledge_id": "k-1",
                    "knowledge_title": "Guide",
                    "knowledge_filename": "guide.pdf",
                    "score": 0.95
                },
                {
                    "content": "second",
                    "knowledge_id": "k-2",
                    "score": 0.1
                },
                { "content": "third" }
            ]
        });
        let hits = parse_search_hits(&body, 2, "weknora");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].content, "first");
        assert_eq!(hits[0].source_id, "k-1");
        assert_eq!(hits[0].title, "Guide");
        assert_eq!(hits[0].filename, "guide.pdf");
        assert_eq!(hits[0].score, Some(0.95));
        assert_eq!(hits[0].provider, "weknora");
        assert_eq!(hits[1].content, "second");
    }

    #[test]
    fn parse_bases_reads_id_and_name() {
        let body = json!({
            "data": [
                { "id": "kb-1", "name": "Lab notes" },
                { "knowledge_base_id": "kb-2", "title": "Papers" }
            ]
        });
        let bases = parse_knowledge_bases(&body);
        assert_eq!(bases[0].id, "kb-1");
        assert_eq!(bases[0].name, "Lab notes");
        assert_eq!(bases[1].id, "kb-2");
        assert_eq!(bases[1].name, "Papers");
    }

    #[test]
    fn http_401_mentions_settings() {
        let message = map_http_error(401, "{\"error\":\"unauthorized\"}");
        assert!(message.contains("401"));
        assert!(message.contains("Knowledge Base"));
        assert!(message.contains("unauthorized"));
    }

    #[test]
    fn unknown_provider_is_rejected() {
        assert!(parse_provider("other").is_err());
        assert_eq!(parse_provider("weknora").unwrap(), Some("weknora"));
        assert_eq!(parse_provider("").unwrap(), None);
    }
}
