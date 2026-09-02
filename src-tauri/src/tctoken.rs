//! TCTOKEN Open API client for account login and the in-app user center.
//!
//! Docs: https://s.apifox.cn/4f329cc7-178e-48d4-b10d-dc3ef55d118d

use crate::AppState;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;
use std::time::Duration;
use superscience_store::secrets::Secret;
use tauri::State;

const DEFAULT_BASE_URL: &str = "https://www.tctoken.cn";

/// Shared TCTOKEN Open API origin. Override with `TCTOKEN_API_BASE` for staging.
pub(crate) fn tctoken_api_base() -> String {
    std::env::var("TCTOKEN_API_BASE")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
}

const ACCESS_TOKEN_SECRET: &str = "tctoken_access_token";
const SESSION_EXPIRED: &str = "Session expired. Please log in again.";
const PROFILE_SETTING: &str = "tctoken_profile";
const REMEMBER_USERNAME_SETTING: &str = "tctoken_remember_username";
const REMEMBER_PASSWORD_SECRET: &str = "tctoken_remember_password";
const DEFAULT_TOKEN_SETTING: &str = "tctoken_default_token_id";
/// Internal quota units per currency unit (docs: ~500000 ≈ $1 / ¥1 display).
const QUOTA_PER_UNIT: f64 = 500_000.0;

fn http_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent("superscience-tctoken/1.0")
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .cookie_store(true)
            .build()
            .expect("build tctoken HTTP client")
    })
}

fn api_url(path: &str) -> String {
    format!("{}{path}", tctoken_api_base())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TctokenSession {
    pub logged_in: bool,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct StoredProfile {
    pub(crate) user_id: i64,
    username: String,
    display_name: String,
    group: String,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    success: bool,
    #[serde(default)]
    message: String,
    data: Option<T>,
}

#[derive(Debug, Default, Deserialize)]
struct LoginData {
    #[serde(default)]
    user_id: Option<i64>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    require_2fa: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TctokenLoginResult {
    pub require_2fa: bool,
    pub session: TctokenSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TctokenRememberedLogin {
    pub remember: bool,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TctokenAccount {
    pub user_id: i64,
    pub username: String,
    pub display_name: String,
    pub group: String,
    pub quota: i64,
    pub used_quota: i64,
    pub request_count: i64,
    pub remaining_display: String,
    pub used_display: String,
}

#[derive(Debug, Deserialize)]
struct AccountData {
    #[serde(default)]
    user_id: i64,
    #[serde(default)]
    username: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    group: String,
    #[serde(default)]
    quota: i64,
    #[serde(default)]
    used_quota: i64,
    #[serde(default)]
    request_count: i64,
}

fn format_quota_cny(quota: i64) -> String {
    format!("¥{:.2}", quota as f64 / QUOTA_PER_UNIT)
}

fn secret_get(name: &str) -> String {
    Secret::get(name).ok().unwrap_or_default()
}

fn secret_set(name: &str, value: &str) -> Result<(), String> {
    Secret::set(name, value).map_err(|e| e.to_string())
}

fn secret_del(name: &str) -> Result<(), String> {
    Secret::delete(name).map_err(|e| e.to_string())
}

pub(crate) async fn load_profile(store: &superscience_store::Store) -> Option<StoredProfile> {
    let raw = store.get_setting(PROFILE_SETTING).await.ok().flatten()?;
    serde_json::from_str(&raw).ok()
}

async fn save_profile(
    store: &superscience_store::Store,
    profile: &StoredProfile,
) -> Result<(), String> {
    let json = serde_json::to_string(profile).map_err(|e| e.to_string())?;
    store
        .set_setting(PROFILE_SETTING, &json)
        .await
        .map_err(|e| e.to_string())
}

async fn clear_profile(store: &superscience_store::Store) -> Result<(), String> {
    store
        .set_setting(PROFILE_SETTING, "")
        .await
        .map_err(|e| e.to_string())
}

async fn load_remembered_login(
    store: &superscience_store::Store,
) -> Result<TctokenRememberedLogin, String> {
    let username = store
        .get_setting(REMEMBER_USERNAME_SETTING)
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_default()
        .trim()
        .to_string();
    let password = secret_get(REMEMBER_PASSWORD_SECRET);
    if username.is_empty() || password.is_empty() {
        return Ok(TctokenRememberedLogin::default());
    }
    Ok(TctokenRememberedLogin {
        remember: true,
        username,
        password,
    })
}

async fn save_remembered_login(
    store: &superscience_store::Store,
    username: &str,
    password: &str,
) -> Result<(), String> {
    let username = username.trim();
    let password = password.trim();
    if username.is_empty() || password.is_empty() {
        return clear_remembered_login(store).await;
    }
    store
        .set_setting(REMEMBER_USERNAME_SETTING, username)
        .await
        .map_err(|e| e.to_string())?;
    secret_set(REMEMBER_PASSWORD_SECRET, password)
}

async fn clear_remembered_login(store: &superscience_store::Store) -> Result<(), String> {
    store
        .set_setting(REMEMBER_USERNAME_SETTING, "")
        .await
        .map_err(|e| e.to_string())?;
    let _ = secret_del(REMEMBER_PASSWORD_SECRET);
    Ok(())
}

fn access_token() -> Result<String, String> {
    let token = secret_get(ACCESS_TOKEN_SECRET);
    if token.trim().is_empty() {
        Err("Not logged in.".into())
    } else {
        Ok(token)
    }
}

fn persist_login(data: &LoginData) -> Result<StoredProfile, String> {
    let token = data
        .access_token
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "Login response missing access_token.".to_string())?;
    secret_set(ACCESS_TOKEN_SECRET, token)?;
    Ok(StoredProfile {
        user_id: data.user_id.unwrap_or_default(),
        username: data.username.clone().unwrap_or_default(),
        display_name: data
            .display_name
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| data.username.clone())
            .unwrap_or_default(),
        group: data.group.clone().unwrap_or_else(|| "default".into()),
    })
}

fn session_from_profile(profile: Option<StoredProfile>, logged_in: bool) -> TctokenSession {
    match profile {
        Some(p) if logged_in => TctokenSession {
            logged_in: true,
            user_id: Some(p.user_id),
            username: Some(p.username),
            display_name: Some(p.display_name),
            group: Some(p.group),
        },
        _ => TctokenSession {
            logged_in: false,
            user_id: None,
            username: None,
            display_name: None,
            group: None,
        },
    }
}

async fn parse_envelope<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T, String> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("read response: {e}"))?;
    let envelope: ApiEnvelope<T> = serde_json::from_str(&body).map_err(|e| {
        if !status.is_success() {
            format!("HTTP {status}: {body}")
        } else {
            format!("invalid response ({e}): {body}")
        }
    })?;
    if !envelope.success {
        let msg = envelope.message.trim();
        return Err(if msg.is_empty() {
            "Request failed.".into()
        } else {
            msg.to_string()
        });
    }
    envelope
        .data
        .ok_or_else(|| "Response missing data.".to_string())
}

pub(crate) fn is_invalid_session_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("unauthorized")
        || lower.contains("invalid access token")
        || lower.contains("session expired")
        || (lower.contains("access token")
            && (lower.contains("invalid")
                || lower.contains("expired")
                || lower.contains("revoked")))
}

async fn clear_local_session(store: Option<&superscience_store::Store>) {
    let _ = secret_del(ACCESS_TOKEN_SECRET);
    if let Some(store) = store {
        let _ = clear_profile(store).await;
    }
}

async fn parse_authed_envelope<T: for<'de> Deserialize<'de>>(
    store: Option<&superscience_store::Store>,
    response: reqwest::Response,
) -> Result<T, String> {
    let status = response.status();
    if matches!(status.as_u16(), 401 | 403) {
        clear_local_session(store).await;
        return Err(SESSION_EXPIRED.into());
    }
    match parse_envelope::<T>(response).await {
        Ok(data) => Ok(data),
        Err(message) if is_invalid_session_error(&message) => {
            clear_local_session(store).await;
            Err(SESSION_EXPIRED.into())
        }
        Err(message) => Err(message),
    }
}

async fn parse_authed_envelope_value(
    store: Option<&superscience_store::Store>,
    response: reqwest::Response,
) -> Result<Value, String> {
    parse_authed_envelope::<Value>(store, response).await
}

async fn authed_get(path: &str) -> Result<reqwest::Response, String> {
    let token = access_token()?;
    http_client()
        .get(api_url(path))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))
}

async fn authed_get_query(
    path: &str,
    query: &[(&str, String)],
) -> Result<reqwest::Response, String> {
    let token = access_token()?;
    http_client()
        .get(api_url(path))
        .query(query)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))
}

async fn authed_post_json(path: &str, body: &Value) -> Result<reqwest::Response, String> {
    let token = access_token()?;
    http_client()
        .post(api_url(path))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(CONTENT_TYPE, "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))
}

#[tauri::command(rename = "tctoken_session")]
pub(super) async fn tctoken_session_cmd(
    state: State<'_, AppState>,
) -> Result<TctokenSession, String> {
    let token = secret_get(ACCESS_TOKEN_SECRET);
    if token.trim().is_empty() {
        return Ok(session_from_profile(None, false));
    }
    let profile = load_profile(&state.store).await;
    Ok(session_from_profile(profile, true))
}

#[tauri::command(rename = "tctoken_login")]
pub(super) async fn tctoken_login_cmd(
    state: State<'_, AppState>,
    username: String,
    password: String,
    refresh_token: Option<bool>,
) -> Result<TctokenLoginResult, String> {
    let username = username.trim().to_string();
    let password = password.trim().to_string();
    if username.is_empty() || password.is_empty() {
        return Err("Username and password are required.".into());
    }
    let body = serde_json::json!({
        "username": username,
        "password": password,
        "refresh_token": refresh_token.unwrap_or(false),
    });
    let response = http_client()
        .post(api_url("/api/open/v1/login"))
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("login request failed: {e}"))?;
    let data: LoginData = parse_envelope(response).await?;
    if data.require_2fa.unwrap_or(false) {
        return Ok(TctokenLoginResult {
            require_2fa: true,
            session: session_from_profile(None, false),
        });
    }
    let profile = persist_login(&data)?;
    save_profile(&state.store, &profile).await?;
    Ok(TctokenLoginResult {
        require_2fa: false,
        session: session_from_profile(Some(profile), true),
    })
}

#[tauri::command(rename = "tctoken_login_2fa")]
pub(super) async fn tctoken_login_2fa_cmd(
    state: State<'_, AppState>,
    code: String,
    refresh_token: Option<bool>,
) -> Result<TctokenLoginResult, String> {
    let code = code.trim().to_string();
    if code.is_empty() {
        return Err("Verification code is required.".into());
    }
    let body = serde_json::json!({
        "code": code,
        "refresh_token": refresh_token.unwrap_or(false),
    });
    let response = http_client()
        .post(api_url("/api/open/v1/login/2fa"))
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("2FA request failed: {e}"))?;
    let data: LoginData = parse_envelope(response).await?;
    let profile = persist_login(&data)?;
    save_profile(&state.store, &profile).await?;
    Ok(TctokenLoginResult {
        require_2fa: false,
        session: session_from_profile(Some(profile), true),
    })
}

#[tauri::command(rename = "tctoken_logout")]
pub(super) async fn tctoken_logout_cmd(
    state: State<'_, AppState>,
) -> Result<TctokenSession, String> {
    let _ = secret_del(ACCESS_TOKEN_SECRET);
    clear_profile(&state.store).await?;
    Ok(session_from_profile(None, false))
}

#[tauri::command(rename = "tctoken_get_remembered_login")]
pub(super) async fn tctoken_get_remembered_login_cmd(
    state: State<'_, AppState>,
) -> Result<TctokenRememberedLogin, String> {
    load_remembered_login(&state.store).await
}

#[tauri::command(rename = "tctoken_set_remembered_login")]
pub(super) async fn tctoken_set_remembered_login_cmd(
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> Result<(), String> {
    save_remembered_login(&state.store, &username, &password).await
}

#[tauri::command(rename = "tctoken_clear_remembered_login")]
pub(super) async fn tctoken_clear_remembered_login_cmd(
    state: State<'_, AppState>,
) -> Result<(), String> {
    clear_remembered_login(&state.store).await
}

#[tauri::command(rename = "tctoken_account")]
pub(super) async fn tctoken_account_cmd(
    state: State<'_, AppState>,
) -> Result<TctokenAccount, String> {
    let response = authed_get("/api/open/v1/account").await?;
    let data: AccountData = parse_authed_envelope(Some(&state.store), response).await?;
    let profile = StoredProfile {
        user_id: data.user_id,
        username: data.username.clone(),
        display_name: if data.display_name.is_empty() {
            data.username.clone()
        } else {
            data.display_name.clone()
        },
        group: data.group.clone(),
    };
    let _ = save_profile(&state.store, &profile).await;
    Ok(TctokenAccount {
        user_id: data.user_id,
        username: data.username,
        display_name: profile.display_name,
        group: data.group,
        quota: data.quota,
        used_quota: data.used_quota,
        request_count: data.request_count,
        remaining_display: format_quota_cny(data.quota),
        used_display: format_quota_cny(data.used_quota),
    })
}

#[tauri::command(rename = "tctoken_logs")]
pub(super) async fn tctoken_logs_cmd(
    p: Option<i64>,
    page_size: Option<i64>,
    log_type: Option<i64>,
    start_timestamp: Option<i64>,
    end_timestamp: Option<i64>,
    token_name: Option<String>,
    model_name: Option<String>,
    group: Option<String>,
) -> Result<Value, String> {
    let mut query = Vec::new();
    query.push(("p", p.unwrap_or(1).to_string()));
    query.push(("page_size", page_size.unwrap_or(20).min(100).to_string()));
    if let Some(t) = log_type {
        query.push(("type", t.to_string()));
    }
    if let Some(v) = start_timestamp {
        query.push(("start_timestamp", v.to_string()));
    }
    if let Some(v) = end_timestamp {
        query.push(("end_timestamp", v.to_string()));
    }
    if let Some(v) = token_name.filter(|s| !s.trim().is_empty()) {
        query.push(("token_name", v));
    }
    if let Some(v) = model_name.filter(|s| !s.trim().is_empty()) {
        query.push(("model_name", v));
    }
    if let Some(v) = group.filter(|s| !s.trim().is_empty()) {
        query.push(("group", v));
    }
    let response = authed_get_query("/api/open/v1/logs", &query).await?;
    parse_authed_envelope_value(None, response).await
}

#[tauri::command(rename = "tctoken_logs_stat")]
pub(super) async fn tctoken_logs_stat_cmd(
    start_timestamp: Option<i64>,
    end_timestamp: Option<i64>,
    token_name: Option<String>,
    model_name: Option<String>,
    group: Option<String>,
    log_type: Option<i64>,
) -> Result<Value, String> {
    let mut query = Vec::new();
    if let Some(v) = start_timestamp {
        query.push(("start_timestamp", v.to_string()));
    }
    if let Some(v) = end_timestamp {
        query.push(("end_timestamp", v.to_string()));
    }
    if let Some(v) = token_name.filter(|s| !s.trim().is_empty()) {
        query.push(("token_name", v));
    }
    if let Some(v) = model_name.filter(|s| !s.trim().is_empty()) {
        query.push(("model_name", v));
    }
    if let Some(v) = group.filter(|s| !s.trim().is_empty()) {
        query.push(("group", v));
    }
    if let Some(t) = log_type {
        query.push(("type", t.to_string()));
    }
    let response = authed_get_query("/api/open/v1/logs/stat", &query).await?;
    parse_authed_envelope_value(None, response).await
}

#[tauri::command(rename = "tctoken_topup_info")]
pub(super) async fn tctoken_topup_info_cmd() -> Result<Value, String> {
    let response = authed_get("/api/open/v1/topup/info").await?;
    parse_authed_envelope_value(None, response).await
}

#[tauri::command(rename = "tctoken_topup_amount")]
pub(super) async fn tctoken_topup_amount_cmd(
    amount: i64,
    payment_method: Option<String>,
) -> Result<Value, String> {
    if amount <= 0 {
        return Err("Amount must be positive.".into());
    }
    let mut body = serde_json::json!({ "amount": amount });
    if let Some(method) = payment_method.filter(|s| !s.trim().is_empty()) {
        body["payment_method"] = Value::String(method);
    }
    let response = authed_post_json("/api/open/v1/topup/amount", &body).await?;
    parse_authed_envelope_value(None, response).await
}

#[tauri::command(rename = "tctoken_topup_pay")]
pub(super) async fn tctoken_topup_pay_cmd(
    amount: i64,
    payment_method: String,
    channel: Option<String>,
) -> Result<Value, String> {
    if amount <= 0 {
        return Err("Amount must be positive.".into());
    }
    let method = payment_method.trim();
    if method.is_empty() {
        return Err("Payment method is required.".into());
    }
    // Apifox Open API: `/api/open/v1/topup/pay`
    // `channel` is only for official Alipay/WeChat gateways. Sending it with
    // 易支付 methods (`alipay` / `wxpay`) can make the server take the wrong path.
    let mut body = serde_json::json!({
        "amount": amount,
        "payment_method": method,
    });
    if matches!(method, "alipay_official" | "wechat_official") {
        let default_channel = if method == "wechat_official" {
            "native"
        } else {
            "pc"
        };
        let ch = channel
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(default_channel);
        body["channel"] = Value::String(ch.to_string());
    }
    let response = authed_post_json("/api/open/v1/topup/pay", &body).await?;
    let mut data = parse_authed_envelope_value(None, response).await?;
    normalize_pay_response(&mut data);
    Ok(data)
}

/// Accept both Open API (`pay_url`) and classic epay (`url` + `data` params) shapes.
fn normalize_pay_response(data: &mut Value) {
    let pay_url = data
        .get("pay_url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    if pay_url.is_some() {
        return;
    }
    if let Some(url) = data
        .get("url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
    {
        data["pay_url"] = Value::String(url);
        return;
    }
    // Some gateways return the checkout URL inside params.
    if let Some(params) = data.get("params").and_then(|v| v.as_object()) {
        for key in ["payurl", "pay_url", "url", "qrcode", "code_url", "mweb_url"] {
            if let Some(url) = params
                .get(key)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data["pay_url"] = Value::String(url.to_string());
                return;
            }
        }
    }
}

#[tauri::command(rename = "tctoken_topup_redeem")]
pub(super) async fn tctoken_topup_redeem_cmd(key: String) -> Result<Value, String> {
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("Redeem code is required.".into());
    }
    let body = serde_json::json!({ "key": key });
    let response = authed_post_json("/api/open/v1/topup/redeem", &body).await?;
    parse_authed_envelope_value(None, response).await
}

#[tauri::command(rename = "tctoken_topup_orders")]
pub(super) async fn tctoken_topup_orders_cmd(
    p: Option<i64>,
    page_size: Option<i64>,
    keyword: Option<String>,
) -> Result<Value, String> {
    let mut query = Vec::new();
    query.push(("p", p.unwrap_or(1).to_string()));
    query.push(("page_size", page_size.unwrap_or(20).min(100).to_string()));
    if let Some(v) = keyword.filter(|s| !s.trim().is_empty()) {
        query.push(("keyword", v));
    }
    let response = authed_get_query("/api/open/v1/topup/orders", &query).await?;
    parse_authed_envelope_value(None, response).await
}

#[tauri::command(rename = "tctoken_tokens")]
pub(super) async fn tctoken_tokens_cmd(
    p: Option<i64>,
    page_size: Option<i64>,
) -> Result<Value, String> {
    let query = [
        ("p", p.unwrap_or(1).to_string()),
        ("page_size", page_size.unwrap_or(20).min(100).to_string()),
    ];
    let response = authed_get_query("/api/open/v1/tokens", &query).await?;
    parse_authed_envelope_value(None, response).await
}

#[tauri::command(rename = "tctoken_token_key")]
pub(super) async fn tctoken_token_key_cmd(id: i64) -> Result<Value, String> {
    let token = access_token()?;
    let response = http_client()
        .post(api_url(&format!("/api/open/v1/tokens/{id}/key")))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;
    parse_authed_envelope_value(None, response).await
}

#[tauri::command(rename = "tctoken_get_default_token_id")]
pub(super) async fn tctoken_get_default_token_id_cmd(
    state: State<'_, AppState>,
) -> Result<Option<i64>, String> {
    let raw = state
        .store
        .get_setting(DEFAULT_TOKEN_SETTING)
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<i64>()
        .map(Some)
        .map_err(|e| format!("invalid default token id: {e}"))
}

#[tauri::command(rename = "tctoken_set_default_token")]
pub(super) async fn tctoken_set_default_token_cmd(
    state: State<'_, AppState>,
    id: i64,
) -> Result<i64, String> {
    if id <= 0 {
        return Err("Token id is required.".into());
    }
    state
        .store
        .set_setting(DEFAULT_TOKEN_SETTING, &id.to_string())
        .await
        .map_err(|e| e.to_string())?;
    // Best-effort: also wire the plaintext key into the image-generation model.
    if let Ok(data) = tctoken_token_key_cmd(id).await {
        if let Some(key) = data
            .get("key")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let _ = crate::models::set_image_generation_api_key(&state.store, key).await;
        }
    }
    Ok(id)
}

#[tauri::command(rename = "tctoken_set_drawing_key")]
pub(super) async fn tctoken_set_drawing_key_cmd(
    state: State<'_, AppState>,
    id: i64,
) -> Result<String, String> {
    let data = tctoken_token_key_cmd(id).await?;
    let key = data
        .get("key")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Token key response missing key.".to_string())?;
    let model_id = crate::models::set_image_generation_api_key(&state.store, key).await?;
    let _ = state
        .store
        .set_setting(DEFAULT_TOKEN_SETTING, &id.to_string())
        .await;
    Ok(model_id)
}

#[tauri::command(rename = "tctoken_provider_url")]
pub(super) fn tctoken_provider_url_cmd() -> String {
    tctoken_api_base()
}

pub(crate) fn ability_card_usage_request_body(
    user_id: i64,
    card_id: &str,
    card_name: &str,
    date: Option<&str>,
) -> Value {
    let mut body = serde_json::json!({
        "user_id": user_id,
        "card_id": card_id,
        "card_name": card_name,
    });
    if let Some(date) = date.map(str::trim).filter(|s| !s.is_empty()) {
        body["date"] = Value::String(date.to_string());
    }
    body
}

pub(crate) async fn report_ability_card_usage(
    user_id: i64,
    card_id: &str,
    card_name: &str,
    date: Option<&str>,
) -> Result<(), String> {
    let body = ability_card_usage_request_body(user_id, card_id, card_name, date);
    let response = authed_post_json("/api/open/v1/ability-cards/usage", &body).await?;
    let _ = parse_authed_envelope_value(None, response).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_quota_as_cny() {
        assert_eq!(format_quota_cny(500_000), "¥1.00");
        assert_eq!(format_quota_cny(214_790_000), "¥429.58");
        assert_eq!(format_quota_cny(0), "¥0.00");
    }

    #[test]
    fn parses_login_success_envelope() {
        let raw = r#"{
            "success": true,
            "message": "",
            "data": {
                "user_id": 1,
                "username": "demo",
                "display_name": "Demo",
                "group": "default",
                "access_token": "tok-abc"
            }
        }"#;
        let env: ApiEnvelope<LoginData> = serde_json::from_str(raw).unwrap();
        assert!(env.success);
        let data = env.data.unwrap();
        assert_eq!(data.access_token.as_deref(), Some("tok-abc"));
        assert_eq!(data.require_2fa, None);
    }

    #[test]
    fn parses_require_2fa_envelope() {
        let raw = r#"{
            "success": true,
            "message": "require 2fa",
            "data": { "require_2fa": true }
        }"#;
        let env: ApiEnvelope<LoginData> = serde_json::from_str(raw).unwrap();
        assert!(env.data.unwrap().require_2fa.unwrap());
    }

    #[test]
    fn api_url_joins_base() {
        let expected = format!("{}/api/open/v1/account", tctoken_api_base());
        assert_eq!(api_url("/api/open/v1/account"), expected);
    }

    #[test]
    fn default_api_base_is_tctoken_cn_when_env_unset() {
        if std::env::var("TCTOKEN_API_BASE").is_ok() {
            return;
        }
        assert_eq!(tctoken_api_base(), DEFAULT_BASE_URL);
    }

    #[test]
    fn ability_card_usage_request_body_includes_optional_date() {
        let body = ability_card_usage_request_body(42, "topic-coach", "选题引导", None);
        assert_eq!(body["user_id"], 42);
        assert_eq!(body["card_id"], "topic-coach");
        assert_eq!(body["card_name"], "选题引导");
        assert!(body.get("date").is_none());

        let dated = ability_card_usage_request_body(1, "handwriting-extract", "手写数据提取", Some("2026-09-02"));
        assert_eq!(dated["date"], "2026-09-02");
    }

    #[test]
    fn detects_invalid_access_token_as_expired_session() {
        assert!(is_invalid_session_error(
            "Unauthorized, invalid access token"
        ));
        assert!(is_invalid_session_error(
            "SESSION EXPIRED. Please log in again."
        ));
        assert!(is_invalid_session_error("access token expired"));
        assert!(!is_invalid_session_error("Amount must be positive."));
        assert!(!is_invalid_session_error(
            "Username and password are required."
        ));
    }

    #[test]
    fn remembered_login_defaults_empty() {
        let blank = TctokenRememberedLogin::default();
        assert!(!blank.remember);
        assert!(blank.username.is_empty());
        assert!(blank.password.is_empty());
    }

    #[test]
    fn normalize_pay_response_promotes_classic_url() {
        let mut data = serde_json::json!({
            "url": "https://pay.example.com/submit",
            "params": { "pid": "1" }
        });
        normalize_pay_response(&mut data);
        assert_eq!(
            data.get("pay_url").and_then(|v| v.as_str()),
            Some("https://pay.example.com/submit")
        );
    }

    #[test]
    fn normalize_pay_response_reads_params_fallback() {
        let mut data = serde_json::json!({
            "params": { "code_url": "weixin://wxpay/bizpayurl?pr=abc" }
        });
        normalize_pay_response(&mut data);
        assert_eq!(
            data.get("pay_url").and_then(|v| v.as_str()),
            Some("weixin://wxpay/bizpayurl?pr=abc")
        );
    }
}
