//! Tauri commands for the first-run / capability local-environment wizard.

use super::{models, AppState};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use superscience_runtime::{
    auto_items_satisfied, probe_local_runtimes, run_provision, ProvisionItem, RealHost,
};
use tauri::{AppHandle, Emitter, Manager, State};

const SETTING_DONE: &str = "runtime_provision_done";

fn cancel_flag() -> &'static AtomicBool {
    static FLAG: OnceLock<AtomicBool> = OnceLock::new();
    FLAG.get_or_init(|| AtomicBool::new(false))
}

fn running_flag() -> &'static AtomicBool {
    static FLAG: OnceLock<AtomicBool> = OnceLock::new();
    FLAG.get_or_init(|| AtomicBool::new(false))
}

fn latest_items() -> &'static Mutex<Vec<ProvisionItem>> {
    static ITEMS: OnceLock<Mutex<Vec<ProvisionItem>>> = OnceLock::new();
    ITEMS.get_or_init(|| Mutex::new(Vec::new()))
}

#[derive(Serialize, Clone)]
pub struct RuntimeProvisionState {
    /// Offered once after first-run welcome. Not an every-launch open flag.
    pub first_run: bool,
    pub done: bool,
    pub running: bool,
    pub items: Vec<ProvisionItem>,
}

#[derive(Serialize, Clone)]
struct RuntimeProvisionProgress {
    items: Vec<ProvisionItem>,
    running: bool,
    current_id: String,
    phase: String,
    received: u64,
    total: Option<u64>,
}

fn has_sci_key() -> bool {
    models::credential_status()
        .into_iter()
        .any(|(id, present)| id == "scimaster_api_key" && present)
}

fn probe_items(app_data: &std::path::Path) -> Vec<ProvisionItem> {
    probe_local_runtimes(
        app_data,
        &RealHost {
            has_sci_key: has_sci_key(),
        },
    )
}

async fn provision_done(store: &superscience_store::Store) -> bool {
    store
        .get_setting(SETTING_DONE)
        .await
        .ok()
        .flatten()
        .is_some()
}

#[tauri::command]
pub(super) async fn get_runtime_provision_state(
    state: State<'_, AppState>,
) -> Result<RuntimeProvisionState, String> {
    let done = provision_done(&state.store).await;
    let items = {
        let cached = latest_items().lock().unwrap();
        if cached.is_empty() {
            drop(cached);
            let probed = probe_items(&state.app_data);
            *latest_items().lock().unwrap() = probed.clone();
            probed
        } else {
            cached.clone()
        }
    };
    Ok(RuntimeProvisionState {
        first_run: !done,
        done,
        running: running_flag().load(Ordering::Relaxed),
        items,
    })
}

#[tauri::command]
pub(super) async fn start_runtime_provision(app: AppHandle) -> Result<(), String> {
    if running_flag().swap(true, Ordering::Relaxed) {
        return Ok(());
    }
    cancel_flag().store(false, Ordering::Relaxed);
    let handle = app.clone();
    let progress_handle = app.clone();
    let app_data = app.state::<AppState>().app_data.clone();
    *latest_items().lock().unwrap() = probe_items(&app_data);
    tauri::async_runtime::spawn(async move {
        let host = RealHost {
            has_sci_key: has_sci_key(),
        };
        let result = tokio::task::spawn_blocking({
            let app_data = app_data.clone();
            move || {
                run_provision(&app_data, &host, cancel_flag(), &{
                    let progress_handle = progress_handle;
                    move |id, phase, received, total| {
                        if let Ok(mut items) = latest_items().lock() {
                            if let Some(item) = items.iter_mut().find(|item| item.id == id) {
                                if phase == "error" {
                                    item.status = "failed".into();
                                } else if phase == "test" {
                                    item.status = "passed".into();
                                } else {
                                    item.status = "installing".into();
                                }
                            }
                            let snapshot = items.clone();
                            drop(items);
                            let _ = progress_handle.emit(
                                "runtime-provision-progress",
                                RuntimeProvisionProgress {
                                    items: snapshot,
                                    running: true,
                                    current_id: id.into(),
                                    phase: phase.into(),
                                    received,
                                    total,
                                },
                            );
                        }
                    }
                })
            }
        })
        .await
        .map_err(|error| error.to_string())
        .and_then(|inner| inner.map_err(|error| error.to_string()));

        running_flag().store(false, Ordering::Relaxed);
        let items = match result {
            Ok(items) => items,
            Err(error) if error == "cancelled" => probe_items(&app_data),
            Err(error) => {
                let mut items = probe_items(&app_data);
                if let Some(item) = items.iter_mut().find(|item| item.status == "installing") {
                    item.status = "failed".into();
                    item.detail = error;
                }
                items
            }
        };
        *latest_items().lock().unwrap() = items.clone();
        apply_bootstrap_from_items(&handle, &items);
        if items
            .iter()
            .any(|item| item.id == "r" && matches!(item.status.as_str(), "ready" | "passed"))
        {
            if let Some(path) =
                superscience_runtime::find_rscript_for_app(&handle.state::<AppState>().app_data)
            {
                let store = handle.state::<AppState>().store.clone();
                let _ = crate::runtime_launcher::save_runtime_interpreter(
                    &store,
                    superscience_runtime::LOCAL_CONTEXT_ID,
                    superscience_runtime::RuntimeLanguage::R,
                    &path.to_string_lossy(),
                )
                .await;
            }
        }
        if auto_items_satisfied(&items) {
            let store = handle.state::<AppState>().store.clone();
            let _ = store.set_setting(SETTING_DONE, "1").await;
        }
        let _ = handle.emit(
            "runtime-provision-progress",
            RuntimeProvisionProgress {
                items,
                running: false,
                current_id: String::new(),
                phase: "done".into(),
                received: 0,
                total: None,
            },
        );
    });
    Ok(())
}

#[tauri::command]
pub(super) fn cancel_runtime_provision() {
    cancel_flag().store(true, Ordering::Relaxed);
}

#[tauri::command]
pub(super) async fn dismiss_runtime_provision(state: State<'_, AppState>) -> Result<(), String> {
    cancel_flag().store(true, Ordering::Relaxed);
    state
        .store
        .set_setting(SETTING_DONE, "1")
        .await
        .map_err(|e| format!("{e}"))
}

#[tauri::command]
pub(super) async fn save_runtime_provision_sci_key(
    state: State<'_, AppState>,
    value: String,
) -> Result<RuntimeProvisionState, String> {
    crate::settings_commands::set_credential(state.clone(), "scimaster_api_key".into(), value)
        .await?;
    let mut items = probe_items(&state.app_data);
    *latest_items().lock().unwrap() = items.clone();
    if let Some(item) = items.iter_mut().find(|item| item.id == "sci_key") {
        item.status = if has_sci_key() {
            "ready".into()
        } else {
            "needs_user".into()
        };
    }
    get_runtime_provision_state(state).await
}

fn apply_bootstrap_from_items(app: &AppHandle, items: &[ProvisionItem]) {
    let state = app.state::<AppState>();
    let mut status = state.bootstrap.lock().unwrap();
    for item in items {
        let ok = matches!(item.status.as_str(), "ready" | "passed");
        match item.id.as_str() {
            "uv" => status.uv_ok = ok,
            "python" => status.python_ok = ok,
            "r" => status.r_ok = ok,
            "node" => status.node_ok = ok,
            "sci" => status.sci_ok = ok,
            "pixi" => status.pixi_ok = ok,
            "officecli" => status.officecli_ok = ok,
            "sci_key" => status.sci_key_ok = ok,
            _ => {}
        }
    }
    let snapshot = status.clone();
    drop(status);
    let _ = app.emit("bootstrap-status", snapshot);
}

#[cfg(test)]
mod tests {
    use super::SETTING_DONE;

    #[test]
    fn provision_done_setting_key_is_stable() {
        assert_eq!(SETTING_DONE, "runtime_provision_done");
    }

    #[test]
    fn provision_state_uses_first_run_not_show() {
        let json = serde_json::to_string(&super::RuntimeProvisionState {
            first_run: true,
            done: false,
            running: false,
            items: vec![],
        })
        .unwrap();
        assert!(json.contains("\"first_run\":true"));
        assert!(!json.contains("\"show\""));
    }
}
