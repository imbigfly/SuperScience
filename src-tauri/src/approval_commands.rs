use super::{save_approval_grants, AppState, ApprovalGrantKey};
use serde::Serialize;
use tauri::State;

async fn ensure_project_frame(
    state: &AppState,
    project_id: &str,
    frame_id: &str,
) -> Result<(), String> {
    match state
        .store
        .frame_project_id(frame_id)
        .await
        .map_err(|error| error.to_string())?
        .as_deref()
    {
        Some(owner) if owner == project_id => Ok(()),
        Some(_) => Err("Conversation does not belong to the active project.".into()),
        None => Err("Conversation does not exist.".into()),
    }
}

pub(crate) fn session_full_permission(state: &AppState, session_id: &str) -> bool {
    state
        .full_permission_sessions
        .read()
        .map(|sessions| sessions.contains(session_id))
        .unwrap_or(false)
}

pub(super) fn cancel_pending_confirmation(state: &AppState, session_id: &str) {
    let pending = state.confirms.lock().unwrap().remove(session_id);
    if let Some(pending) = pending {
        let _ = pending
            .tx
            .send(superscience_tools::ConfirmDecision::Denied { feedback: None });
    }
    state.awaiting_confirm.lock().unwrap().remove(session_id);
    state.device_hub.resolve_needs_user(session_id);
}

#[tauri::command]
pub(super) async fn get_session_full_permission(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    session_id: String,
) -> Result<bool, String> {
    let project = state.active(window.label());
    ensure_project_frame(&state, &project.id, &session_id).await?;
    Ok(session_full_permission(&state, &session_id))
}

#[tauri::command]
pub(super) async fn set_session_full_permission(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    session_id: String,
    enabled: bool,
) -> Result<bool, String> {
    let project = state.active(window.label());
    ensure_project_frame(&state, &project.id, &session_id).await?;
    {
        let mut sessions = state
            .full_permission_sessions
            .write()
            .map_err(|_| "Full Permission state is unavailable.".to_string())?;
        if enabled {
            sessions.insert(session_id.clone());
        } else {
            sessions.remove(&session_id);
        }
    }

    // If the user enables the mode while an ordinary approval is already
    // waiting, settle that approval immediately. Later confirmation sites read
    // the shared mode live and never enqueue a card.
    if enabled {
        let pending = state.confirms.lock().unwrap().remove(&session_id);
        if let Some(pending) = pending {
            let _ = pending
                .tx
                .send(superscience_tools::ConfirmDecision::Approved);
            state.awaiting_confirm.lock().unwrap().remove(&session_id);
            state.device_hub.resolve_needs_user(&session_id);
        }
    }
    Ok(enabled)
}

#[tauri::command]
pub(super) async fn confirm_response(
    state: State<'_, AppState>,
    session_id: String,
    approved: bool,
    feedback: Option<String>,
    scope: Option<String>,
) -> Result<(), String> {
    let decision = if approved {
        superscience_tools::ConfirmDecision::Approved
    } else {
        superscience_tools::ConfirmDecision::Denied {
            feedback: feedback
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        }
    };
    let pending = state.confirms.lock().unwrap().remove(&session_id);
    if let Some(pending) = pending {
        state.device_hub.resolve_needs_user(&session_id);
        // Deliver the live decision before persistence: a disk failure must not
        // strand the agent on a removed confirm channel (especially "global" /
        // "project" grants, which write SQLite).
        let _ = pending.tx.send(decision);
        if approved {
            let scope = scope.unwrap_or_else(|| "once".into());
            if matches!(scope.as_str(), "session" | "project" | "global") {
                if let Some(grant) = pending.grant.clone() {
                    let snapshot = {
                        let mut grants = state.approval_grants.lock().unwrap();
                        grants.grant(&scope, &session_id, &pending.project_id, grant);
                        grants.clone()
                    };
                    if scope != "session" {
                        if let Err(error) = save_approval_grants(&state.store, &snapshot).await {
                            tracing::warn!(
                                target: "superscience",
                                error = %error,
                                scope = %scope,
                                "failed to persist approval grant after confirm"
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    } else {
        Err("no pending confirmation".into())
    }
}

#[derive(Serialize, Clone)]
pub(super) struct ApprovalGrantInfo {
    scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,
    kind: String,
    target: String,
    label: String,
}

fn approval_grant_label(key: &ApprovalGrantKey) -> String {
    match key.target.as_str() {
        "shell" => "Shell commands".into(),
        other => other.to_string(),
    }
}

#[tauri::command]
pub(super) fn list_approval_grants(state: State<'_, AppState>) -> Vec<ApprovalGrantInfo> {
    let grants = state.approval_grants.lock().unwrap().clone();
    let mut out = vec![];
    for (session_id, keys) in grants.session {
        for key in keys {
            out.push(ApprovalGrantInfo {
                scope: "session".into(),
                session_id: Some(session_id.clone()),
                project_id: None,
                label: approval_grant_label(&key),
                kind: key.kind,
                target: key.target,
            });
        }
    }
    for (project_id, keys) in grants.project {
        for key in keys {
            out.push(ApprovalGrantInfo {
                scope: "project".into(),
                session_id: None,
                project_id: Some(project_id.clone()),
                label: approval_grant_label(&key),
                kind: key.kind,
                target: key.target,
            });
        }
    }
    for key in grants.global {
        out.push(ApprovalGrantInfo {
            scope: "global".into(),
            session_id: None,
            project_id: None,
            label: approval_grant_label(&key),
            kind: key.kind,
            target: key.target,
        });
    }
    out.sort_by(|a, b| {
        a.scope
            .cmp(&b.scope)
            .then(a.label.cmp(&b.label))
            .then(a.target.cmp(&b.target))
    });
    out
}

#[tauri::command]
pub(super) async fn revoke_approval_grant(
    state: State<'_, AppState>,
    scope: String,
    kind: String,
    target: String,
    session_id: Option<String>,
    project_id: Option<String>,
) -> Result<(), String> {
    let key = ApprovalGrantKey { kind, target };
    let snapshot = {
        let mut grants = state.approval_grants.lock().unwrap();
        grants.revoke(&scope, session_id.as_deref(), project_id.as_deref(), &key);
        grants.clone()
    };
    save_approval_grants(&state.store, &snapshot).await
}

#[tauri::command]
pub(super) async fn revoke_all_approval_grants(state: State<'_, AppState>) -> Result<(), String> {
    let snapshot = {
        let mut grants = state.approval_grants.lock().unwrap();
        grants.clear();
        grants.clone()
    };
    save_approval_grants(&state.store, &snapshot).await
}
