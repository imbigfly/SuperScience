//! Loopback bridge to a user-installed Chrome/Chromium extension.
//!
//! The extension runs inside the user's existing browser profile, so page
//! execution keeps that profile's cookies, login state, extensions, GPU, and
//! browser fingerprint. Wisp never launches a separate automation browser.
//!
//! Design acknowledgement: this bridge is inspired by GenericAgent's GA Web /
//! TMWebDriver real-browser architecture and compatible loopback protocol:
//! https://github.com/lsdefine/GenericAgent (MIT, Copyright 2025 lsdefine).
//! This module is Wisp's independent Rust implementation; see
//! `browser-extension/NOTICE.md` for provenance details.

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use superscience_llm::ToolSchema;
use superscience_store::Store;
use superscience_tools::{Approval, ImageData, Tool, ToolEnv, ToolResult};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_hdr_async, WebSocketStream};
use uuid::Uuid;

use crate::browser_url_filters::{self, BrowserUrlFilters};

const BRIDGE_ADDR: &str = "127.0.0.1:18765";
const EXTENSION_ORIGIN: &str = "chrome-extension://gnkjgagleagkgdlkkcianolobfdoocnp";
const BROWSER_DISCONNECTED_CODE: &str = "browser_extension_disconnected";
const BROWSER_DISCONNECTED_MARKER: &str = "WISP_BROWSER_DISCONNECTED";
const DISCONNECTED_ASSISTANT_INSTRUCTION: &str = "Live web retrieval is unavailable. Do not answer live, latest, current, or URL-specific questions from prior knowledge. Tell the user this turn contains no live web retrieval, relay the install steps, and wait until status is connected. Only continue from memory if they explicitly ask for a knowledge-only answer.";
const DEFAULT_TIMEOUT_MS: u64 = 15_000;
const MAX_TIMEOUT_MS: u64 = 60_000;
const MAX_SCRIPT_BYTES: usize = 64 * 1024;
const MAX_RESULT_CHARS: usize = 200_000;
/// Base64 payload ceiling for one screenshot, matching the shared image path's
/// 5 MB decoded limit (base64 inflates by 4/3).
const MAX_SCREENSHOT_B64: usize = 7 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct BrowserTab {
    id: i64,
    url: String,
    title: String,
    active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    window_id: Option<i64>,
}

#[derive(Clone)]
struct BridgeClient {
    connection_id: u64,
    tx: mpsc::UnboundedSender<Message>,
}

#[derive(Default)]
struct BridgeState {
    client: Option<BridgeClient>,
    tabs: BTreeMap<i64, BrowserTab>,
    selected_tab: Option<i64>,
    pending: HashMap<String, oneshot::Sender<Result<BridgeReply, String>>>,
    startup_error: Option<String>,
}

pub struct BrowserBridge {
    state: Mutex<BridgeState>,
    next_connection_id: AtomicU64,
    extension_dir: PathBuf,
}

#[derive(Clone, Debug)]
struct BridgeReply {
    value: Value,
    ready: Option<bool>,
    wait: Option<Value>,
}

struct BrowserExecution {
    tab_id: i64,
    value: Value,
    ready: Option<bool>,
    wait: Option<Value>,
}

impl BrowserBridge {
    fn new(extension_dir: PathBuf) -> Self {
        Self {
            state: Mutex::new(BridgeState::default()),
            next_connection_id: AtomicU64::new(1),
            extension_dir,
        }
    }

    pub async fn start(extension_dir: PathBuf) -> Arc<Self> {
        let bridge = Arc::new(Self::new(extension_dir));
        match TcpListener::bind(BRIDGE_ADDR).await {
            Ok(listener) => {
                let task_bridge = bridge.clone();
                tokio::spawn(async move { task_bridge.accept_loop(listener).await });
            }
            Err(error) => {
                bridge.state.lock().await.startup_error = Some(format!(
                    "cannot listen on {BRIDGE_ADDR}: {error}; stop any other TMWebDriver/SuperScience browser bridge using this port"
                ));
            }
        }
        bridge
    }

    async fn setup_info(&self) -> Value {
        let state = self.state.lock().await;
        let extension_path = self.verified_extension_path();
        let extension_ready = extension_path.is_some();
        // A live extension connection is the only proof that live retrieval
        // works. It outranks an unverifiable bundled copy: a user who loaded the
        // extension from another folder still browses fine, and reporting
        // extension_missing there told the model and the UI "no live retrieval"
        // on every turn (#921).
        let status = if state.client.is_some() {
            "connected"
        } else if state.startup_error.is_some() {
            "error"
        } else if !extension_ready {
            "extension_missing"
        } else {
            "disconnected"
        };
        let live_retrieval = status == "connected";
        let steps = extension_path.as_ref().map_or_else(Vec::new, |path| {
            vec![
                "Start SuperScience and keep it running.".to_string(),
                "Open chrome://extensions in the Chrome/Chromium profile SuperScience should control."
                    .to_string(),
                "Enable Developer mode.".to_string(),
                format!("Click Load unpacked and select this exact folder: {path}"),
                "Open the SuperScience Real Browser Bridge extension popup and confirm Connected to SuperScience."
                    .to_string(),
            ]
        });
        let path_instruction = if extension_ready {
            "Copy extension_path character-for-character. Never translate, infer, normalize, or replace any path segment."
        } else {
            "The running SuperScience build has no verified bundled extension path. Do not invent a path or claim the extension exists."
        };
        let assistant_instruction = if live_retrieval {
            path_instruction.to_string()
        } else {
            format!("{DISCONNECTED_ASSISTANT_INSTRUCTION} {path_instruction}")
        };

        json!({
            "status": status,
            "live_retrieval": live_retrieval,
            "code": if live_retrieval { Value::Null } else { json!(BROWSER_DISCONNECTED_CODE) },
            "connected_tabs": state.tabs.len(),
            "runtime_os": std::env::consts::OS,
            "path_source": "wisp_tauri_resource_dir",
            "extension_path": extension_path,
            "extension_path_verified": extension_ready,
            "extension_id": EXTENSION_ORIGIN.trim_start_matches("chrome-extension://"),
            "bridge_endpoint": format!("ws://{BRIDGE_ADDR}"),
            "install_scope": "once_per_browser_profile",
            "assistant_instruction": assistant_instruction,
            "steps": steps,
            "download_automation": {
                "limitation": "GA Web controls web-page tabs. It cannot operate Chrome/Edge toolbar download bubbles or native operating-system Open, Save, and Save As dialogs.",
                "manual_setup_required": true,
                "chrome_settings_url": "chrome://settings/downloads",
                "edge_settings_url": "edge://settings/downloads",
                "setting_to_disable": "Ask where to save each file before downloading",
                "multiple_downloads": {
                    "chrome_settings_url": "chrome://settings/content/automaticDownloads",
                    "edge_settings_url": "edge://settings/content/automaticDownloads",
                    "agent_gate": "Before triggering multiple file downloads, show these browser settings and wait for the user to confirm configuration. Until confirmed, download at most one file.",
                    "recommended_action": "Add only the trusted target site to Allowed to automatically download multiple files. If the browser asks on the site's first batch, choose Allow.",
                    "security_note": "Do not allow automatic multiple downloads for untrusted sites."
                },
                "effect": "Downloads save to the browser's configured default download directory without opening a native location prompt. Authorized filesystem tools may process the saved file afterward."
            },
            "error": state.startup_error
        })
    }

    async fn accept_loop(self: Arc<Self>, listener: TcpListener) {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let bridge = self.clone();
                    tokio::spawn(async move {
                        if let Err(error) = bridge.accept_connection(stream).await {
                            tracing::warn!(target: "wisp", "browser bridge connection rejected: {error}");
                        }
                    });
                }
                Err(error) => {
                    tracing::warn!(target: "wisp", "browser bridge accept failed: {error}");
                }
            }
        }
    }

    async fn accept_connection(
        self: Arc<Self>,
        stream: TcpStream,
    ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        let socket = accept_hdr_async(stream, |request: &Request, response: Response| {
            let origin = request
                .headers()
                .get("origin")
                .and_then(|value| value.to_str().ok());
            if allowed_extension_origin(origin) {
                Ok(response)
            } else {
                Err(forbidden_response())
            }
        })
        .await?;
        self.serve_connection(socket).await;
        Ok(())
    }

    async fn serve_connection(self: Arc<Self>, socket: WebSocketStream<TcpStream>) {
        let connection_id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);
        let (mut writer, mut reader) = socket.split();
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.install_client(connection_id, tx.clone()).await;
        let writer_task = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if writer.send(message).await.is_err() {
                    break;
                }
            }
        });

        while let Some(message) = reader.next().await {
            match message {
                Ok(Message::Text(text)) => self.handle_text(connection_id, text.as_str()).await,
                Ok(Message::Ping(payload)) => {
                    let _ = tx.send(Message::Pong(payload));
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
        writer_task.abort();
        self.disconnect_client(connection_id).await;
    }

    async fn install_client(&self, connection_id: u64, tx: mpsc::UnboundedSender<Message>) {
        let mut state = self.state.lock().await;
        fail_pending(&mut state, "browser extension connection was replaced");
        state.client = Some(BridgeClient { connection_id, tx });
        state.tabs.clear();
        state.selected_tab = None;
    }

    async fn disconnect_client(&self, connection_id: u64) {
        let mut state = self.state.lock().await;
        if state
            .client
            .as_ref()
            .is_some_and(|client| client.connection_id == connection_id)
        {
            state.client = None;
            state.tabs.clear();
            state.selected_tab = None;
            fail_pending(&mut state, "browser extension disconnected");
        }
    }

    async fn handle_text(&self, connection_id: u64, text: &str) {
        let Ok(message) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let message_type = message.get("type").and_then(Value::as_str).unwrap_or("");
        let mut state = self.state.lock().await;
        if !state
            .client
            .as_ref()
            .is_some_and(|client| client.connection_id == connection_id)
        {
            return;
        }
        match message_type {
            "ext_ready" | "tabs_update" => replace_tabs(&mut state, &message),
            "result" | "error" => {
                let Some(id) = message.get("id").and_then(Value::as_str) else {
                    return;
                };
                let Some(sender) = state.pending.remove(id) else {
                    return;
                };
                let result = if message_type == "result" {
                    Ok(parse_bridge_reply(&message))
                } else {
                    Err(render_bridge_error(message.get("error")))
                };
                let _ = sender.send(result);
            }
            _ => {}
        }
    }

    async fn execute(
        &self,
        requested_tab: Option<i64>,
        code: &str,
        timeout: Duration,
    ) -> Result<BrowserExecution, String> {
        let id = Uuid::new_v4().to_string();
        let (response_tx, response_rx) = oneshot::channel();
        let tab_id = {
            let mut state = self.state.lock().await;
            if let Some(error) = &state.startup_error {
                return Err(self.unavailable_message(error));
            }
            let Some(client) = state.client.clone() else {
                return Err(self.unavailable_message("browser extension is not connected"));
            };
            let tab_id = select_tab(&state, requested_tab)?;
            state.selected_tab = Some(tab_id);
            state.pending.insert(id.clone(), response_tx);
            let payload = request_payload(&id, Some(tab_id), code, timeout);
            if client.tx.send(Message::Text(payload.into())).is_err() {
                state.pending.remove(&id);
                return Err("browser extension disconnected before the request was sent".into());
            }
            tab_id
        };

        match tokio::time::timeout(timeout, response_rx).await {
            Ok(Ok(Ok(reply))) => Ok(BrowserExecution {
                tab_id,
                value: reply.value,
                ready: reply.ready,
                wait: reply.wait,
            }),
            Ok(Ok(Err(error))) => Err(error),
            Ok(Err(_)) => Err("browser extension disconnected before returning a result".into()),
            Err(_) => {
                self.state.lock().await.pending.remove(&id);
                Err(format!(
                    "browser execution timed out after {} ms",
                    timeout.as_millis()
                ))
            }
        }
    }

    /// Send a control command that does not target an existing tab (e.g. open a
    /// new tab). Unlike `execute`, this never requires an HTTP(S) tab to exist,
    /// so it can bootstrap browsing from an empty profile.
    async fn send_command(&self, code: String, timeout: Duration) -> Result<BridgeReply, String> {
        let id = Uuid::new_v4().to_string();
        let (response_tx, response_rx) = oneshot::channel();
        {
            let mut state = self.state.lock().await;
            if let Some(error) = &state.startup_error {
                return Err(self.unavailable_message(error));
            }
            let Some(client) = state.client.clone() else {
                return Err(self.unavailable_message("browser extension is not connected"));
            };
            state.pending.insert(id.clone(), response_tx);
            let payload = request_payload(&id, None, &code, timeout);
            if client.tx.send(Message::Text(payload.into())).is_err() {
                state.pending.remove(&id);
                return Err("browser extension disconnected before the request was sent".into());
            }
        }

        match tokio::time::timeout(timeout, response_rx).await {
            Ok(Ok(Ok(value))) => Ok(value),
            Ok(Ok(Err(error))) => Err(error),
            Ok(Err(_)) => Err("browser extension disconnected before returning a result".into()),
            Err(_) => {
                self.state.lock().await.pending.remove(&id);
                Err(format!(
                    "browser command timed out after {} ms",
                    timeout.as_millis()
                ))
            }
        }
    }

    async fn open_tab(&self, url: &str, active: bool) -> Result<BridgeReply, String> {
        let code =
            json!({ "cmd": "tabs", "method": "create", "url": url, "active": active }).to_string();
        self.send_command(code, Duration::from_millis(DEFAULT_TIMEOUT_MS))
            .await
    }

    async fn tabs(&self) -> Result<Vec<BrowserTab>, String> {
        let state = self.state.lock().await;
        if let Some(error) = &state.startup_error {
            return Err(self.unavailable_message(error));
        }
        if state.client.is_none() {
            return Err(self.unavailable_message("browser extension is not connected"));
        }
        Ok(state.tabs.values().cloned().collect())
    }

    fn unavailable_message(&self, reason: &str) -> String {
        let setup = match self.verified_extension_path() {
            Some(path) => format!(
                "real-browser bridge unavailable: {reason}. In Chrome/Chromium open chrome://extensions, enable Developer mode, and Load unpacked from this exact native {} path: '{path}'. Keep SuperScience running; the extension connects only to {BRIDGE_ADDR}.",
                std::env::consts::OS
            ),
            None => format!(
                "real-browser bridge unavailable: {reason}. This SuperScience build has no verified bundled browser extension; do not infer an installation path."
            ),
        };
        format!("{setup} {BROWSER_DISCONNECTED_MARKER}. {DISCONNECTED_ASSISTANT_INSTRUCTION}")
    }

    fn verified_extension_path(&self) -> Option<String> {
        let dir = dunce::canonicalize(&self.extension_dir).ok()?;
        (dir.join("manifest.json").is_file() && dir.join("wait_tab.js").is_file())
            .then(|| dir.display().to_string())
    }
}

fn allowed_extension_origin(origin: Option<&str>) -> bool {
    origin == Some(EXTENSION_ORIGIN)
}

fn forbidden_response() -> ErrorResponse {
    tokio_tungstenite::tungstenite::http::Response::builder()
        .status(StatusCode::FORBIDDEN)
        .body(Some(
            "SuperScience browser bridge accepts Chrome extensions only".into(),
        ))
        .expect("static browser bridge rejection response")
}

fn fail_pending(state: &mut BridgeState, reason: &str) {
    for (_, sender) in state.pending.drain() {
        let _ = sender.send(Err(reason.to_string()));
    }
}

fn replace_tabs(state: &mut BridgeState, message: &Value) {
    let Some(tabs) = message.get("tabs").and_then(Value::as_array) else {
        return;
    };
    state.tabs = tabs
        .iter()
        .filter_map(parse_tab)
        .map(|tab| (tab.id, tab))
        .collect();
    if !state
        .selected_tab
        .is_some_and(|tab_id| state.tabs.contains_key(&tab_id))
    {
        state.selected_tab = state
            .tabs
            .values()
            .find(|tab| tab.active)
            .or_else(|| state.tabs.values().next())
            .map(|tab| tab.id);
    }
}

fn parse_tab(value: &Value) -> Option<BrowserTab> {
    let id = value.get("id").and_then(|id| {
        id.as_i64()
            .or_else(|| id.as_str().and_then(|id| id.parse().ok()))
    })?;
    Some(BrowserTab {
        id,
        url: value
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        active: value
            .get("active")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        window_id: value.get("windowId").and_then(Value::as_i64),
    })
}

fn select_tab(state: &BridgeState, requested: Option<i64>) -> Result<i64, String> {
    if state.tabs.is_empty() {
        return Err("browser extension is connected, but no HTTP(S) tabs are available".into());
    }
    if let Some(tab_id) = requested {
        return state
            .tabs
            .contains_key(&tab_id)
            .then_some(tab_id)
            .ok_or_else(|| {
                format!("browser tab {tab_id} is not available; call web_scan with tabs_only=true")
            });
    }
    state
        .selected_tab
        .filter(|tab_id| state.tabs.contains_key(tab_id))
        .or_else(|| state.tabs.values().find(|tab| tab.active).map(|tab| tab.id))
        .or_else(|| state.tabs.keys().next().copied())
        .ok_or_else(|| "no browser tab is selected".into())
}

fn request_payload(id: &str, tab_id: Option<i64>, code: &str, timeout: Duration) -> String {
    let mut payload = json!({
        "id": id,
        "code": code,
        "timeoutMs": timeout.as_millis() as u64,
    });
    if let Some(tab_id) = tab_id {
        payload["tabId"] = json!(tab_id);
    }
    payload.to_string()
}

fn parse_bridge_reply(message: &Value) -> BridgeReply {
    BridgeReply {
        value: message
            .get("result")
            .or_else(|| message.get("data"))
            .cloned()
            .unwrap_or(Value::Null),
        ready: message.get("ready").and_then(Value::as_bool),
        wait: message
            .get("wait")
            .cloned()
            .filter(|value| !value.is_null()),
    }
}

fn merge_ready_wait(mut payload: Value, ready: Option<bool>, wait: Option<Value>) -> Value {
    if let Some(ready) = ready {
        payload["ready"] = json!(ready);
    }
    if let Some(wait) = wait {
        payload["wait"] = wait;
    }
    payload
}

fn render_bridge_error(error: Option<&Value>) -> String {
    match error {
        Some(Value::String(error)) => error.clone(),
        Some(error) => serde_json::to_string_pretty(error).unwrap_or_else(|_| error.to_string()),
        None => "browser extension returned an unknown error".into(),
    }
}

fn tab_id_arg(args: &Value) -> Result<Option<i64>, String> {
    let Some(value) = args.get("switch_tab_id") else {
        return Ok(None);
    };
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        .map(Some)
        .ok_or_else(|| "switch_tab_id must be an integer tab id returned by web_scan".into())
}

fn open_tab_result(reply: BridgeReply, url: &str, filters: &BrowserUrlFilters) -> Value {
    let mut result = json!({ "tab": reply.value });
    if !filters.prefer.is_empty() {
        let preferred = filters.is_preferred(url);
        result["preferred"] = json!(preferred);
        if !preferred {
            result["prefer_hosts"] = json!(filters
                .prefer
                .iter()
                .map(|rule| rule.host.clone())
                .collect::<Vec<_>>());
        }
    }
    merge_ready_wait(result, reply.ready, reply.wait)
}

fn render_json(value: &Value) -> String {
    let rendered = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    if rendered.chars().count() <= MAX_RESULT_CHARS {
        return rendered;
    }
    let mut clipped: String = rendered.chars().take(MAX_RESULT_CHARS).collect();
    clipped.push_str("\n... browser result truncated");
    clipped
}

fn human_verification_handoff(page: &Value) -> Option<Value> {
    let title = page.get("title").and_then(Value::as_str).unwrap_or("");
    let text = page.get("text").and_then(Value::as_str).unwrap_or("");
    let content = format!("{title}\n{text}").to_ascii_lowercase();
    if !content.contains("are you a robot")
        || !(content.contains("confirm you are a human") || content.contains("captcha challenge"))
    {
        return None;
    }
    Some(json!({
        "required": true,
        "reason": "captcha_challenge",
        "instruction": "Stop browser automation and ask the user to complete the human-verification challenge manually in this current visible browser tab. Wait for the user to confirm completion before scanning the same tab again.",
        "resume": "After the user confirms, call web_scan on the same tab and continue only when the challenge is no longer detected."
    }))
}

const SCAN_SCRIPT: &str = r##"(() => {
  const visible = (el) => {
    const s = getComputedStyle(el), r = el.getBoundingClientRect();
    return s.display !== 'none' && s.visibility !== 'hidden' && Number(s.opacity) > 0 && r.width > 0 && r.height > 0;
  };
  const selector = (el) => {
    if (el.id) {
      const id = '#' + CSS.escape(el.id);
      if (document.querySelectorAll(id).length === 1) return id;
    }
    const parts = [];
    for (let node = el; node && node.nodeType === 1 && parts.length < 6; node = node.parentElement) {
      let part = node.tagName.toLowerCase();
      const siblings = node.parentElement ? [...node.parentElement.children].filter(x => x.tagName === node.tagName) : [];
      if (siblings.length > 1) part += `:nth-of-type(${siblings.indexOf(node) + 1})`;
      parts.unshift(part);
      const candidate = parts.join(' > ');
      if (document.querySelectorAll(candidate).length === 1) return candidate;
    }
    return parts.join(' > ');
  };
  const query = 'a,button,input,textarea,select,summary,[role],[contenteditable=true],h1,h2,h3,label';
  const elements = [...document.querySelectorAll(query)].filter(visible).slice(0, 400).map((el) => {
    const r = el.getBoundingClientRect(), type = el.getAttribute('type') || '';
    return {
      selector: selector(el), tag: el.tagName.toLowerCase(), role: el.getAttribute('role') || undefined,
      text: (el.innerText || el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 500) || undefined,
      aria_label: el.getAttribute('aria-label') || undefined, href: el.href || undefined, type: type || undefined,
      value: type.toLowerCase() === 'password' ? undefined : (el.value || undefined), disabled: !!el.disabled,
      rect: [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)]
    };
  });
  return { url: location.href, title: document.title, viewport: [innerWidth, innerHeight],
    ready_state: document.readyState,
    text: (document.body?.innerText || '').slice(0, 30000), elements };
})()"##;

const TEXT_SCAN_SCRIPT: &str = r#"(() => ({
  url: location.href,
  title: document.title,
  ready_state: document.readyState,
  text: (document.body?.innerText || '').slice(0, 50000)
}))()"#;

pub struct BrowserSetupTool {
    bridge: Arc<BrowserBridge>,
    store: Store,
}

impl BrowserSetupTool {
    pub fn new(bridge: Arc<BrowserBridge>, store: Store) -> Self {
        Self { bridge, store }
    }
}

#[async_trait]
impl Tool for BrowserSetupTool {
    fn name(&self) -> &str {
        "browser_setup"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            "Call when the user asks to configure, install, set up, or connect the real browser, and before any live page retrieval. The result is derived from the running SuperScience binary's native Tauri resource directory and includes the manual settings required for unattended single and multiple downloads. Copy extension_path character-for-character and never convert it between Windows, WSL, macOS, or Linux. If status is not connected, live_retrieval is false: do not answer live, latest, current, or URL-specific questions from prior knowledge; relay the steps and wait. If extension_path_verified is false, report the missing bundled extension and never invent a path.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        )
    }

    fn preview(&self, _args: &Value) -> String {
        "show real-browser setup status and extension path".into()
    }

    async fn run(&self, _args: &Value, _env: &dyn ToolEnv) -> ToolResult {
        let mut info = self.bridge.setup_info().await;
        let filters = browser_url_filters::load(&self.store).await;
        info["url_filters"] = json!({
            "block": filters.block,
            "prefer": filters.prefer,
            "matching": "host and subdomains; block is enforced; prefer is advisory for literature and similar tasks"
        });
        ToolResult::ok(render_json(&info))
    }
}

pub struct WebScanTool {
    bridge: Arc<BrowserBridge>,
}

impl WebScanTool {
    pub fn new(bridge: Arc<BrowserBridge>) -> Self {
        Self { bridge }
    }
}

#[async_trait]
impl Tool for WebScanTool {
    fn name(&self) -> &str {
        "web_scan"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            "Read visible content and actionable elements from the user's real, persistent Chrome/Chromium session. The browser keeps its existing cookies, login state, extensions, GPU/WebGL behavior, and normal profile fingerprint. Waits until the tab's document is complete before reading (or until timeout). The result includes ready and page.ready_state; if ready is false, scan again instead of clicking a partial page. Use tabs_only first when the target tab is unclear. If the result contains human_intervention.required=true, stop browser automation, ask the user to complete the challenge in the current visible tab, and wait for confirmation before scanning again.",
            json!({
                "type": "object",
                "properties": {
                    "tabs_only": { "type": "boolean", "description": "List connected HTTP(S) tabs without reading page content" },
                    "switch_tab_id": { "type": ["integer", "string"], "description": "Tab id returned by this tool; selects that tab for this and later calls" },
                    "text_only": { "type": "boolean", "description": "Return page text without the actionable-element snapshot" }
                }
            }),
        )
    }

    fn minimum_approval(&self) -> Approval {
        Approval::Ask
    }

    fn preview(&self, args: &Value) -> String {
        if args
            .get("tabs_only")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "list real-browser tabs".into()
        } else {
            args.get("switch_tab_id")
                .map(|tab| format!("scan real-browser tab {tab}"))
                .unwrap_or_else(|| "scan selected real-browser tab".into())
        }
    }

    async fn run(&self, args: &Value, _env: &dyn ToolEnv) -> ToolResult {
        if args
            .get("tabs_only")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return match self.bridge.tabs().await {
                Ok(tabs) => ToolResult::ok(render_json(&json!({ "tabs": tabs }))),
                Err(error) => ToolResult::fail(error),
            };
        }
        let tab_id = match tab_id_arg(args) {
            Ok(tab_id) => tab_id,
            Err(error) => return ToolResult::fail(error),
        };
        let text_only = args
            .get("text_only")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        match self
            .bridge
            .execute(
                tab_id,
                if text_only {
                    TEXT_SCAN_SCRIPT
                } else {
                    SCAN_SCRIPT
                },
                Duration::from_millis(DEFAULT_TIMEOUT_MS),
            )
            .await
        {
            Ok(execution) => {
                let handoff = human_verification_handoff(&execution.value);
                ToolResult::ok(render_json(&merge_ready_wait(
                    json!({
                        "human_intervention": handoff,
                        "tab_id": execution.tab_id,
                        "page": execution.value
                    }),
                    execution.ready,
                    execution.wait,
                )))
            }
            Err(error) => ToolResult::fail(error),
        }
    }
}

pub struct WebExecuteJsTool {
    bridge: Arc<BrowserBridge>,
    store: Store,
}

impl WebExecuteJsTool {
    pub fn new(bridge: Arc<BrowserBridge>, store: Store) -> Self {
        Self { bridge, store }
    }
}

#[async_trait]
impl Tool for WebExecuteJsTool {
    fn name(&self) -> &str {
        "web_execute_js"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            "Execute JavaScript in a tab from the user's real, persistent Chrome/Chromium session. The extension waits until the tab's document is complete before running the script, and waits again if the script navigates. The result includes ready; if ready is false, scan again before clicking. Call web_scan first and do not guess selectors. To close tabs, never call window.close(); send {\"cmd\":\"tabs\",\"method\":\"close\",\"tabIds\":[...]} using ids returned by web_open_tab/web_scan. If web_scan reports human_intervention.required=true, do not automate the challenge; wait for the user to complete it and confirm before continuing. For a task that will trigger multiple file downloads, first tell the user how to allow automatic multiple downloads for the trusted target site at chrome://settings/content/automaticDownloads or edge://settings/content/automaticDownloads, then wait for confirmation; until confirmed, trigger at most one file download. A JSON script with cmd='cdp' may call one Chrome DevTools Protocol method for trusted input or other advanced browser actions.",
            json!({
                "type": "object",
                "properties": {
                    "script": { "type": "string", "description": "JavaScript, or a JSON command such as {\"cmd\":\"cdp\",\"method\":\"Input.dispatchMouseEvent\",\"params\":{...}}" },
                    "switch_tab_id": { "type": ["integer", "string"], "description": "Tab id returned by web_scan" },
                    "timeout_ms": { "type": "integer", "minimum": 1, "maximum": 60000, "description": "Execution timeout in milliseconds (default 15000)" }
                },
                "required": ["script"]
            }),
        )
    }

    fn minimum_approval(&self) -> Approval {
        Approval::Ask
    }

    fn preview(&self, args: &Value) -> String {
        let script = args.get("script").and_then(Value::as_str).unwrap_or("");
        let mut preview: String = script.chars().take(240).collect();
        if script.chars().count() > 240 {
            preview.push('…');
        }
        preview
    }

    async fn run(&self, args: &Value, _env: &dyn ToolEnv) -> ToolResult {
        let Some(script) = args
            .get("script")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|script| !script.is_empty())
        else {
            return ToolResult::fail("missing required argument 'script'");
        };
        if script.len() > MAX_SCRIPT_BYTES {
            return ToolResult::fail(format!(
                "browser script is {} bytes (maximum {MAX_SCRIPT_BYTES})",
                script.len()
            ));
        }
        if script.starts_with("window.close()") {
            return ToolResult::fail(
                "window.close() cannot close ordinary browser tabs. Use the browser-use skill's tab command: {\"cmd\":\"tabs\",\"method\":\"close\",\"tabIds\":[...]} with tab ids returned by web_open_tab/web_scan.",
            );
        }
        let filters = browser_url_filters::load(&self.store).await;
        if let Some((url, rule)) = filters.blocked_navigation(script) {
            return ToolResult::fail(browser_url_filters::block_message(&url, rule));
        }
        let tab_id = match tab_id_arg(args) {
            Ok(tab_id) => tab_id,
            Err(error) => return ToolResult::fail(error),
        };
        let timeout_ms = args
            .get("timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TIMEOUT_MS)
            .clamp(1, MAX_TIMEOUT_MS);
        match self
            .bridge
            .execute(tab_id, script, Duration::from_millis(timeout_ms))
            .await
        {
            Ok(execution) => ToolResult::ok(render_json(&merge_ready_wait(
                json!({
                    "tab_id": execution.tab_id,
                    "result": execution.value
                }),
                execution.ready,
                execution.wait,
            ))),
            Err(error) => ToolResult::fail(error),
        }
    }
}

pub struct WebOpenTabTool {
    bridge: Arc<BrowserBridge>,
    store: Store,
}

impl WebOpenTabTool {
    pub fn new(bridge: Arc<BrowserBridge>, store: Store) -> Self {
        Self { bridge, store }
    }
}

#[async_trait]
impl Tool for WebOpenTabTool {
    fn name(&self) -> &str {
        "web_open_tab"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            "Open a new tab at an http(s) URL in the user's real, persistent Chrome/Chromium session. Works even when no tab is open yet, so use this to start browsing. Waits until the new tab's document is complete (or until timeout). The result includes the new tab id plus ready; if ready is false, call web_scan before acting. Pass the tab id as switch_tab_id to web_scan or web_execute_js. User-defined blocked hosts from Settings → Browser are refused before the tab opens. When url_filters.prefer is non-empty, prefer those hosts for literature and similar retrieval.",
            json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Absolute http:// or https:// URL to open" },
                    "active": { "type": "boolean", "description": "Focus the new tab (default false)" }
                },
                "required": ["url"]
            }),
        )
    }

    fn minimum_approval(&self) -> Approval {
        Approval::Ask
    }

    fn preview(&self, args: &Value) -> String {
        let url = args.get("url").and_then(Value::as_str).unwrap_or("");
        format!("open real-browser tab at {url}")
    }

    async fn run(&self, args: &Value, _env: &dyn ToolEnv) -> ToolResult {
        let Some(url) = args
            .get("url")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|url| !url.is_empty())
        else {
            return ToolResult::fail("missing required argument 'url'");
        };
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return ToolResult::fail("url must be an absolute http:// or https:// address");
        }
        let filters = browser_url_filters::load(&self.store).await;
        if let Some(rule) = filters.blocked(url) {
            return ToolResult::fail(browser_url_filters::block_message(url, rule));
        }
        let active = args.get("active").and_then(Value::as_bool).unwrap_or(false);
        match self.bridge.open_tab(url, active).await {
            Ok(reply) => ToolResult::ok(render_json(&open_tab_result(reply, url, &filters))),
            Err(error) => ToolResult::fail(error),
        }
    }
}

pub struct WebScreenshotTool {
    bridge: Arc<BrowserBridge>,
}

impl WebScreenshotTool {
    pub fn new(bridge: Arc<BrowserBridge>) -> Self {
        Self { bridge }
    }
}

#[async_trait]
impl Tool for WebScreenshotTool {
    fn name(&self) -> &str {
        "web_screenshot"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            "Look at what a tab in the user's real Chrome/Chromium session is showing. Waits until the tab's document is complete before capturing. Use it when web_scan's text and element snapshot is not enough: rendered layout, a chart or diagram, a canvas/WebGL page, a QR code, a PDF or image viewer, or a page that looks wrong and needs eyes. Captures the visible viewport of the tab; to reach content below the fold, scroll with web_execute_js first and capture again. Pass 'question' to say what should be read out of the screenshot.",
            json!({
                "type": "object",
                "properties": {
                    "switch_tab_id": { "type": ["integer", "string"], "description": "Tab id returned by web_scan; selects that tab for this and later calls" },
                    "question": { "type": "string", "description": "What to look for in the screenshot, e.g. 'is the login QR code visible and not expired?'" }
                }
            }),
        )
    }

    fn minimum_approval(&self) -> Approval {
        Approval::Ask
    }

    fn preview(&self, args: &Value) -> String {
        args.get("switch_tab_id")
            .map(|tab| format!("screenshot real-browser tab {tab}"))
            .unwrap_or_else(|| "screenshot selected real-browser tab".into())
    }

    async fn run(&self, args: &Value, _env: &dyn ToolEnv) -> ToolResult {
        let tab_id = match tab_id_arg(args) {
            Ok(tab_id) => tab_id,
            Err(error) => return ToolResult::fail(error),
        };
        // ponytail: reuses the extension's existing CDP path, so no new
        // permission and no extension change. JPEG q80 keeps a viewport well
        // under the image limit and still resolves QR codes and small text.
        let code = json!({
            "cmd": "cdp",
            "method": "Page.captureScreenshot",
            "params": { "format": "jpeg", "quality": 80 }
        })
        .to_string();
        let execution = match self
            .bridge
            .execute(tab_id, &code, Duration::from_millis(DEFAULT_TIMEOUT_MS))
            .await
        {
            Ok(execution) => execution,
            Err(error) => return ToolResult::fail(error),
        };
        let Some(data) = execution
            .value
            .get("data")
            .and_then(Value::as_str)
            .filter(|data| !data.is_empty())
        else {
            return ToolResult::fail("browser screenshot returned no image data");
        };
        if data.len() > MAX_SCREENSHOT_B64 {
            return ToolResult::fail(format!(
                "browser screenshot is too large ({} bytes of base64); reduce the browser window size and retry",
                data.len()
            ));
        }
        ToolResult::image(ImageData {
            mime: "image/jpeg".into(),
            data_url: format!("data:image/jpeg;base64,{data}"),
            label: format!(
                "Screenshot of real browser tab {} ({} KB)",
                execution.tab_id,
                data.len() / 1024
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use sha2::{Digest, Sha256};

    struct NoEnv(PathBuf);

    #[async_trait]
    impl ToolEnv for NoEnv {
        fn project_root(&self) -> &std::path::Path {
            &self.0
        }

        async fn confirm(&self, _message: &str) -> bool {
            true
        }

        async fn emit(&self, _event: superscience_tools::ToolEvent) {}
    }

    async fn empty_store() -> (Store, PathBuf) {
        let tmp =
            std::env::temp_dir().join(format!("wisp_browser_tool_{}.sqlite", uuid::Uuid::new_v4()));
        (Store::open(&tmp).await.unwrap(), tmp)
    }

    #[test]
    fn manifest_key_matches_the_only_accepted_extension_origin() {
        let manifest_path = superscience_paths::browser_extension_dir()
            .unwrap()
            .join("manifest.json");
        let manifest: Value =
            serde_json::from_slice(&std::fs::read(manifest_path).unwrap()).unwrap();
        let key = manifest["key"].as_str().unwrap();
        let der = base64::engine::general_purpose::STANDARD
            .decode(key)
            .unwrap();
        let digest = Sha256::digest(der);
        let id: String = digest[..16]
            .iter()
            .flat_map(|byte| [byte >> 4, byte & 0x0f])
            .map(|nibble| char::from(b'a' + nibble))
            .collect();
        assert_eq!(EXTENSION_ORIGIN, format!("chrome-extension://{id}"));
    }

    #[test]
    fn bridge_accepts_extension_origins_only() {
        assert!(allowed_extension_origin(Some(
            "chrome-extension://gnkjgagleagkgdlkkcianolobfdoocnp"
        )));
        assert!(!allowed_extension_origin(Some(
            "chrome-extension://abcdefghijklmnop"
        )));
        assert!(!allowed_extension_origin(Some("https://example.com")));
        assert!(!allowed_extension_origin(Some("null")));
        assert!(!allowed_extension_origin(None));
    }

    #[tokio::test]
    async fn page_access_tools_always_require_approval() {
        let bridge = Arc::new(BrowserBridge::new(PathBuf::from("extension")));
        let (store, tmp) = empty_store().await;
        assert_eq!(
            WebScanTool::new(bridge.clone()).minimum_approval(),
            Approval::Ask
        );
        assert_eq!(
            WebExecuteJsTool::new(bridge.clone(), store).minimum_approval(),
            Approval::Ask
        );
        assert_eq!(
            WebScreenshotTool::new(bridge).minimum_approval(),
            Approval::Ask
        );
        let _ = std::fs::remove_file(tmp);
    }

    #[tokio::test]
    async fn execute_js_rejects_window_close_with_the_tab_command() {
        let bridge = Arc::new(BrowserBridge::new(PathBuf::from("extension")));
        let (store, tmp) = empty_store().await;
        let result = WebExecuteJsTool::new(bridge, store)
            .run(
                &json!({ "script": "window.close(); 'close-requested'" }),
                &NoEnv(PathBuf::from(".")),
            )
            .await;
        assert!(!result.success);
        assert!(result.content.contains("\"method\":\"close\""));
        assert!(result.content.contains("tabIds"));
        let _ = std::fs::remove_file(tmp);
    }

    #[test]
    fn are_you_a_robot_page_requires_manual_handoff() {
        let handoff = human_verification_handoff(&json!({
            "title": "Are you a robot?",
            "text": "Please confirm you are a human by completing the captcha challenge below."
        }))
        .unwrap();

        assert_eq!(handoff["required"], true);
        assert_eq!(handoff["reason"], "captcha_challenge");
        assert!(handoff["instruction"]
            .as_str()
            .unwrap()
            .contains("Wait for the user to confirm"));
        assert!(human_verification_handoff(&json!({
            "title": "Browser automation article",
            "text": "This article asks: Are you a robot?"
        }))
        .is_none());
    }

    #[tokio::test]
    async fn setup_reports_the_extension_folder_without_requiring_approval() {
        let extension_dir = superscience_paths::browser_extension_dir().unwrap();
        let bridge = Arc::new(BrowserBridge::new(extension_dir.clone()));
        let (store, tmp) = empty_store().await;
        let info = bridge.setup_info().await;
        let expected_path = dunce::canonicalize(extension_dir).unwrap();

        assert_eq!(info["status"], "disconnected");
        assert_eq!(info["live_retrieval"], false);
        assert_eq!(info["code"], BROWSER_DISCONNECTED_CODE);
        assert!(info["assistant_instruction"]
            .as_str()
            .unwrap()
            .contains("Do not answer live, latest, current, or URL-specific questions"));
        assert_eq!(info["runtime_os"], std::env::consts::OS);
        assert_eq!(info["path_source"], "wisp_tauri_resource_dir");
        assert_eq!(info["extension_path"], expected_path.display().to_string());
        assert_eq!(info["extension_path_verified"], true);
        assert_eq!(info["install_scope"], "once_per_browser_profile");
        assert_eq!(
            info["download_automation"]["chrome_settings_url"],
            "chrome://settings/downloads"
        );
        assert_eq!(
            info["download_automation"]["setting_to_disable"],
            "Ask where to save each file before downloading"
        );
        assert_eq!(
            info["download_automation"]["multiple_downloads"]["chrome_settings_url"],
            "chrome://settings/content/automaticDownloads"
        );
        assert!(
            info["download_automation"]["multiple_downloads"]["recommended_action"]
                .as_str()
                .unwrap()
                .contains("trusted target site")
        );
        assert!(
            info["download_automation"]["multiple_downloads"]["agent_gate"]
                .as_str()
                .unwrap()
                .contains("wait for the user to confirm")
        );
        assert!(WebExecuteJsTool::new(bridge.clone(), store.clone())
            .schema()
            .function
            .description
            .contains("until confirmed, trigger at most one file download"));
        assert!(info["steps"].as_array().unwrap().iter().any(|step| step
            .as_str()
            .unwrap()
            .contains(info["extension_path"].as_str().unwrap())));
        let unavailable = bridge.unavailable_message("not connected");
        assert!(unavailable.contains(info["extension_path"].as_str().unwrap()));
        assert!(unavailable.contains(BROWSER_DISCONNECTED_MARKER));
        assert!(
            unavailable.contains("Do not answer live, latest, current, or URL-specific questions")
        );
        assert_eq!(
            BrowserSetupTool::new(bridge, store).minimum_approval(),
            Approval::Allow
        );
        let _ = std::fs::remove_file(tmp);
    }

    #[tokio::test]
    async fn setup_never_offers_an_unverified_extension_path() {
        let missing = std::env::temp_dir().join(format!(
            "wisp-browser-extension-missing-{}",
            std::process::id()
        ));
        let bridge = BrowserBridge::new(missing.clone());
        let info = bridge.setup_info().await;

        assert_eq!(info["status"], "extension_missing");
        assert_eq!(info["live_retrieval"], false);
        assert_eq!(info["code"], BROWSER_DISCONNECTED_CODE);
        assert_eq!(info["extension_path_verified"], false);
        assert!(info["extension_path"].is_null());
        assert!(info["steps"].as_array().unwrap().is_empty());
        assert!(!bridge
            .unavailable_message("not connected")
            .contains(&missing.display().to_string()));

        let incomplete = std::env::temp_dir().join(format!(
            "wisp-browser-extension-incomplete-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&incomplete).unwrap();
        std::fs::write(incomplete.join("manifest.json"), "{}").unwrap();
        let incomplete_bridge = BrowserBridge::new(incomplete.clone());
        let incomplete_info = incomplete_bridge.setup_info().await;
        assert_eq!(incomplete_info["status"], "extension_missing");
        assert!(incomplete_info["extension_path"].is_null());
        let _ = std::fs::remove_dir_all(&incomplete);
    }

    #[test]
    fn tab_parser_accepts_generic_agent_numeric_and_string_ids() {
        let numeric =
            parse_tab(&json!({ "id": 7, "url": "https://a", "title": "A", "active": true }))
                .unwrap();
        let string = parse_tab(&json!({ "id": "8", "url": "https://b", "title": "B" })).unwrap();
        assert_eq!(numeric.id, 7);
        assert!(numeric.active);
        assert_eq!(string.id, 8);
    }

    #[tokio::test]
    async fn routes_execution_to_the_live_extension_and_correlates_result() {
        let bridge = Arc::new(BrowserBridge::new(PathBuf::from("extension")));
        let (tx, mut rx) = mpsc::unbounded_channel();
        bridge.install_client(1, tx).await;
        bridge
            .handle_text(
                1,
                r#"{"type":"ext_ready","tabs":[{"id":42,"url":"https://example.com","title":"Example","active":true}]}"#,
            )
            .await;

        let running = {
            let bridge = bridge.clone();
            tokio::spawn(async move {
                bridge
                    .execute(None, "document.title", Duration::from_secs(1))
                    .await
            })
        };
        let outbound = rx.recv().await.unwrap().into_text().unwrap();
        let outbound: Value = serde_json::from_str(&outbound).unwrap();
        assert_eq!(outbound["tabId"], 42);
        assert_eq!(outbound["code"], "document.title");
        assert_eq!(outbound["timeoutMs"], 1000);
        let id = outbound["id"].as_str().unwrap();
        bridge
            .handle_text(
                1,
                &json!({ "type": "result", "id": id, "result": "Example" }).to_string(),
            )
            .await;

        let result = running.await.unwrap().unwrap();
        assert_eq!(result.tab_id, 42);
        assert_eq!(result.value, "Example");
    }

    #[tokio::test]
    async fn open_tab_creates_a_tab_without_any_existing_tab() {
        let bridge = Arc::new(BrowserBridge::new(PathBuf::from("extension")));
        let (tx, mut rx) = mpsc::unbounded_channel();
        bridge.install_client(1, tx).await;
        // No tabs_update sent: state.tabs is empty, yet open_tab must still work.

        let running = {
            let bridge = bridge.clone();
            tokio::spawn(async move { bridge.open_tab("https://example.com", true).await })
        };
        let outbound = rx.recv().await.unwrap().into_text().unwrap();
        let outbound: Value = serde_json::from_str(&outbound).unwrap();
        assert!(outbound.get("tabId").is_none());
        assert_eq!(outbound["timeoutMs"], DEFAULT_TIMEOUT_MS);
        let command: Value = serde_json::from_str(outbound["code"].as_str().unwrap()).unwrap();
        assert_eq!(command["cmd"], "tabs");
        assert_eq!(command["method"], "create");
        assert_eq!(command["url"], "https://example.com");
        assert_eq!(command["active"], true);

        let id = outbound["id"].as_str().unwrap();
        bridge
            .handle_text(
                1,
                &json!({ "type": "result", "id": id, "result": { "id": 99, "url": "https://example.com" } })
                    .to_string(),
            )
            .await;

        let reply = running.await.unwrap().unwrap();
        assert_eq!(reply.value["id"], 99);
        assert!(reply.ready.is_none());
        assert!(reply.wait.is_none());
    }

    #[tokio::test]
    async fn open_tab_and_execute_js_refuse_blocked_hosts() {
        let bridge = Arc::new(BrowserBridge::new(PathBuf::from("extension")));
        let (store, tmp) = empty_store().await;
        crate::browser_url_filters::save(
            &store,
            BrowserUrlFilters {
                block: vec![crate::browser_url_filters::BrowserUrlFilterRule {
                    host: "blocked.test".into(),
                    reason: "hijacked".into(),
                }],
                prefer: vec![crate::browser_url_filters::BrowserUrlFilterRule {
                    host: "pubmed.ncbi.nlm.nih.gov".into(),
                    reason: String::new(),
                }],
            },
        )
        .await
        .unwrap();

        let opened = WebOpenTabTool::new(bridge.clone(), store.clone())
            .run(
                &json!({ "url": "https://www.blocked.test/paper" }),
                &NoEnv(PathBuf::from(".")),
            )
            .await;
        assert!(!opened.success);
        assert!(opened.content.contains("hijacked"));
        assert!(opened.content.contains("blocked.test"));

        let navigated = WebExecuteJsTool::new(bridge.clone(), store.clone())
            .run(
                &json!({ "script": "location.href='https://blocked.test/js'" }),
                &NoEnv(PathBuf::from(".")),
            )
            .await;
        assert!(!navigated.success);
        assert!(navigated.content.contains("blocked by user URL filter"));

        let setup = BrowserSetupTool::new(bridge, store)
            .run(&json!({}), &NoEnv(PathBuf::from(".")))
            .await;
        assert!(setup.success);
        assert!(setup.content.contains("blocked.test"));
        assert!(setup.content.contains("pubmed.ncbi.nlm.nih.gov"));
        let _ = std::fs::remove_file(tmp);
    }

    #[test]
    fn open_tab_result_flags_non_preferred_hosts() {
        let filters = BrowserUrlFilters {
            prefer: vec![crate::browser_url_filters::BrowserUrlFilterRule {
                host: "pubmed.ncbi.nlm.nih.gov".into(),
                reason: String::new(),
            }],
            ..BrowserUrlFilters::default()
        };
        let flagged = open_tab_result(
            BridgeReply {
                value: json!({ "id": 1 }),
                ready: None,
                wait: None,
            },
            "https://scholar.google.com",
            &filters,
        );
        assert_eq!(flagged["preferred"], false);
        assert_eq!(flagged["prefer_hosts"][0], "pubmed.ncbi.nlm.nih.gov");
        let preferred = open_tab_result(
            BridgeReply {
                value: json!({ "id": 1 }),
                ready: Some(true),
                wait: Some(json!({ "until": "complete", "waited_ms": 12 })),
            },
            "https://pubmed.ncbi.nlm.nih.gov/1",
            &filters,
        );
        assert_eq!(preferred["ready"], true);
        assert_eq!(preferred["wait"]["waited_ms"], 12);
        assert_eq!(preferred["preferred"], true);
        assert!(preferred.get("prefer_hosts").is_none());
    }

    #[tokio::test]
    async fn scan_and_open_surface_ready_wait_from_the_extension() {
        let bridge = Arc::new(BrowserBridge::new(PathBuf::from("extension")));
        let (tx, mut rx) = mpsc::unbounded_channel();
        bridge.install_client(1, tx).await;
        bridge
            .handle_text(
                1,
                r#"{"type":"ext_ready","tabs":[{"id":7,"url":"https://example.com","title":"Example","active":true}]}"#,
            )
            .await;

        let scanning = {
            let bridge = bridge.clone();
            tokio::spawn(async move {
                WebScanTool::new(bridge)
                    .run(&json!({}), &NoEnv(PathBuf::from(".")))
                    .await
            })
        };
        let outbound = rx.recv().await.unwrap().into_text().unwrap();
        let outbound: Value = serde_json::from_str(&outbound).unwrap();
        assert_eq!(outbound["timeoutMs"], DEFAULT_TIMEOUT_MS);
        let id = outbound["id"].as_str().unwrap();
        bridge
            .handle_text(
                1,
                &json!({
                    "type": "result",
                    "id": id,
                    "result": {
                        "url": "https://example.com",
                        "title": "Example",
                        "ready_state": "complete",
                        "text": "hello",
                        "elements": []
                    },
                    "ready": true,
                    "wait": { "until": "complete", "waited_ms": 80, "status": "complete" }
                })
                .to_string(),
            )
            .await;
        let scanned = scanning.await.unwrap();
        assert!(scanned.success);
        let body: Value = serde_json::from_str(&scanned.content).unwrap();
        assert_eq!(body["ready"], true);
        assert_eq!(body["wait"]["waited_ms"], 80);
        assert_eq!(body["page"]["ready_state"], "complete");

        let opening = {
            let bridge = bridge.clone();
            tokio::spawn(async move { bridge.open_tab("https://example.com/paper", false).await })
        };
        let outbound = rx.recv().await.unwrap().into_text().unwrap();
        let outbound: Value = serde_json::from_str(&outbound).unwrap();
        let id = outbound["id"].as_str().unwrap();
        bridge
            .handle_text(
                1,
                &json!({
                    "type": "result",
                    "id": id,
                    "result": { "id": 8, "url": "https://example.com/paper", "title": "Paper", "status": "loading" },
                    "ready": false,
                    "wait": { "until": "complete", "waited_ms": 14500, "timed_out": true, "status": "loading" }
                })
                .to_string(),
            )
            .await;
        let opened = opening.await.unwrap().unwrap();
        assert_eq!(opened.ready, Some(false));
        assert_eq!(opened.wait.as_ref().unwrap()["timed_out"], true);
        let rendered = open_tab_result(
            opened,
            "https://example.com/paper",
            &BrowserUrlFilters::default(),
        );
        assert_eq!(rendered["ready"], false);
        assert_eq!(rendered["tab"]["id"], 8);
    }

    #[test]
    fn scan_scripts_report_document_ready_state() {
        assert!(SCAN_SCRIPT.contains("ready_state: document.readyState"));
        assert!(TEXT_SCAN_SCRIPT.contains("ready_state: document.readyState"));
    }

    #[test]
    fn wait_tab_complete_matches_documented_contract() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../browser-extension");
        let output = std::process::Command::new("node")
            .args(["--test", "wait_tab.test.mjs"])
            .current_dir(&dir)
            .output()
            .expect("node --test should run the extension waiter contract");
        assert!(
            output.status.success(),
            "node --test wait_tab.test.mjs failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn browser_results_are_bounded_before_entering_model_context() {
        let rendered = render_json(&json!({ "text": "x".repeat(MAX_RESULT_CHARS * 2) }));
        assert!(rendered.chars().count() <= MAX_RESULT_CHARS + 40);
        assert!(rendered.ends_with("browser result truncated"));
    }

    #[tokio::test]
    async fn only_bridge_unavailability_carries_the_disconnect_marker() {
        let bridge = Arc::new(BrowserBridge::new(PathBuf::from("extension")));
        assert!(bridge
            .unavailable_message("browser extension is not connected")
            .contains(BROWSER_DISCONNECTED_MARKER));
        // A tab-level failure is not a disconnect and must not raise the
        // "no live retrieval" banner.
        let (tx, _rx) = mpsc::unbounded_channel();
        bridge.install_client(1, tx).await;
        let result = WebScanTool::new(bridge)
            .run(&json!({ "switch_tab_id": 9 }), &NoEnv(PathBuf::from(".")))
            .await;
        assert!(!result.success);
        assert!(!result.content.contains(BROWSER_DISCONNECTED_MARKER));
    }

    struct RecordingEnv {
        root: PathBuf,
        events: std::sync::Mutex<Vec<superscience_tools::ToolEvent>>,
    }

    #[async_trait]
    impl ToolEnv for RecordingEnv {
        fn project_root(&self) -> &std::path::Path {
            &self.root
        }

        async fn confirm(&self, _message: &str) -> bool {
            true
        }

        async fn emit(&self, event: superscience_tools::ToolEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    /// The tool result itself is the live-retrieval record the UI reads. A
    /// separate presentation event outlived the turn it described and revived a
    /// stale "no live retrieval" banner (#887, #921), so browser tools emit none.
    #[tokio::test]
    async fn disconnected_tools_mark_the_result_without_a_presentation() {
        let bridge = Arc::new(BrowserBridge::new(PathBuf::from("extension")));
        let env = RecordingEnv {
            root: PathBuf::from("."),
            events: std::sync::Mutex::new(Vec::new()),
        };
        let result = WebScanTool::new(bridge)
            .run(&json!({ "tabs_only": true }), &env)
            .await;
        assert!(!result.success);
        assert!(result.content.contains(BROWSER_DISCONNECTED_MARKER));
        assert!(env.events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_connected_extension_outranks_an_unverifiable_bundled_copy() {
        let missing = std::env::temp_dir().join(format!(
            "wisp-browser-extension-connected-{}",
            uuid::Uuid::new_v4()
        ));
        let bridge = Arc::new(BrowserBridge::new(missing));
        let (tx, _rx) = mpsc::unbounded_channel();
        bridge.install_client(1, tx).await;
        let info = bridge.setup_info().await;

        assert_eq!(info["status"], "connected");
        assert_eq!(info["live_retrieval"], true);
        assert!(info["code"].is_null());
        assert_eq!(info["extension_path_verified"], false);
    }
}
