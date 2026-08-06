use crate::app_support::process_item_insert_index;
use crate::bindings::invoke_checked;
use crate::dto::ChatItem;
use leptos::*;
use serde_wasm_bindgen::to_value;
use std::collections::HashMap;

pub(crate) fn acp_value_text(value: Option<&serde_json::Value>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(text) => text.clone(),
        value => serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
    }
}

pub(crate) fn upsert_acp_tool(items: &mut Vec<ChatItem>, payload: &serde_json::Value) {
    let Some(call_id) = payload
        .get("toolCallId")
        .and_then(serde_json::Value::as_str)
    else {
        return;
    };
    let index = items
        .iter()
        .position(|item| matches!(item, ChatItem::AcpTool { call_id: id, .. } if id == call_id));
    if let Some(index) = index {
        if let ChatItem::AcpTool {
            title,
            kind,
            status,
            content,
            locations,
            ..
        } = &mut items[index]
        {
            if let Some(value) = payload.get("title").and_then(serde_json::Value::as_str) {
                *title = value.into();
            }
            if let Some(value) = payload.get("kind").and_then(serde_json::Value::as_str) {
                *kind = value.into();
            }
            if let Some(value) = payload.get("status").and_then(serde_json::Value::as_str) {
                *status = value.into();
            }
            if payload.get("content").is_some() {
                *content = acp_value_text(payload.get("content"));
            }
            if payload.get("locations").is_some() {
                *locations = acp_value_text(payload.get("locations"));
            }
        }
    } else {
        let row = ChatItem::AcpTool {
            call_id: call_id.into(),
            title: payload
                .get("title")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("ACP tool")
                .into(),
            kind: payload
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .into(),
            status: payload
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("pending")
                .into(),
            content: acp_value_text(payload.get("content")),
            locations: acp_value_text(payload.get("locations")),
        };
        let index = process_item_insert_index(items);
        items.insert(index, row);
    }
}

/// ACP has no flag marking a mode as "plans without executing"; agents name it
/// themselves (`plan`, `plan_mode`, …), so match the id.
pub(crate) fn is_plan_mode_id(id: &str) -> bool {
    id.to_ascii_lowercase().contains("plan")
}

/// `(plan mode, mode to return to)` from a session's `availableModes`, or `None`
/// for agents that expose no plan mode — the plan toggle stays hidden for those.
pub(crate) fn plan_mode_pair(state: Option<&serde_json::Value>) -> Option<(String, String)> {
    let ids: Vec<&str> = state?
        .get("availableModes")?
        .as_array()?
        .iter()
        .filter_map(|mode| mode.get("id")?.as_str())
        .collect();
    let plan = ids.iter().find(|id| is_plan_mode_id(id))?;
    let exit = ids
        .iter()
        .find(|id| **id == "default")
        .or_else(|| ids.iter().find(|id| !is_plan_mode_id(id)))?;
    Some((plan.to_string(), exit.to_string()))
}

/// What the plan card's action bar was asked to do. All three end plan mode
/// except `Modify`, which keeps it so the agent re-plans from the composer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanDecision {
    Approve,
    Modify,
    SaveExit,
}

pub(crate) fn acp_current_mode_id(state: Option<&serde_json::Value>) -> Option<&str> {
    state?.get("currentModeId")?.as_str()
}

#[cfg(test)]
mod plan_mode_tests {
    use super::plan_mode_pair;

    fn state(ids: &[&str]) -> serde_json::Value {
        serde_json::json!({
            "availableModes": ids.iter().map(|id| serde_json::json!({ "id": id })).collect::<Vec<_>>()
        })
    }

    #[test]
    fn pairs_plan_mode_with_the_mode_to_return_to() {
        let modes = state(&["default", "plan", "acceptEdits"]);
        assert_eq!(
            plan_mode_pair(Some(&modes)),
            Some(("plan".into(), "default".into()))
        );
        // No `default`: fall back to the first mode that is not a plan mode.
        let modes = state(&["plan_mode", "yolo"]);
        assert_eq!(
            plan_mode_pair(Some(&modes)),
            Some(("plan_mode".into(), "yolo".into()))
        );
    }

    #[test]
    fn no_pair_means_no_plan_toggle() {
        assert_eq!(plan_mode_pair(None), None);
        assert_eq!(plan_mode_pair(Some(&state(&["default", "yolo"]))), None);
        assert_eq!(plan_mode_pair(Some(&state(&["plan"]))), None);
        assert_eq!(plan_mode_pair(Some(&serde_json::json!({}))), None);
    }
}

/// `session/set_mode` returns no state, so the applied id is merged locally to
/// preserve the `availableModes` captured from the initial SessionModeState.
/// Returns whether the agent accepted the switch.
pub(crate) async fn apply_acp_mode(
    modes: RwSignal<HashMap<String, serde_json::Value>>,
    frame_id: String,
    mode_id: String,
) -> bool {
    let args = to_value(&serde_json::json!({
        "frameId": frame_id.clone(),
        "modeId": mode_id,
    }))
    .unwrap();
    if let Ok(value) = invoke_checked("set_acp_session_mode", args).await {
        if let Some(applied) = value.as_string() {
            modes.update(|all| {
                let entry = all.entry(frame_id).or_insert_with(|| serde_json::json!({}));
                if let serde_json::Value::Object(map) = entry {
                    map.insert("currentModeId".into(), serde_json::Value::String(applied));
                }
            });
            return true;
        }
    }
    // Notify anyway so a control that already moved optimistically (the plan
    // checkbox, the mode select) snaps back to the mode the agent is really in.
    modes.update(|_| {});
    false
}

pub(crate) fn acp_select_options(option: &serde_json::Value) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for row in option
        .get("options")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(value) = row.get("value").and_then(serde_json::Value::as_str) {
            result.push((
                value.into(),
                row.get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(value)
                    .into(),
            ));
        } else if let Some(options) = row.get("options").and_then(serde_json::Value::as_array) {
            for choice in options {
                if let Some(value) = choice.get("value").and_then(serde_json::Value::as_str) {
                    result.push((
                        value.into(),
                        choice
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or(value)
                            .into(),
                    ));
                }
            }
        }
    }
    result
}

pub(crate) fn remove_optimistic_send_rows(rows: &mut Vec<ChatItem>, display_message: &str) {
    let Some(index) = rows.iter().rposition(|item| {
        matches!(item, ChatItem::User(value) if value == display_message)
            || matches!(item, ChatItem::QueuedUser { text, .. } if text == display_message)
    }) else {
        return;
    };
    if matches!(rows.get(index), Some(ChatItem::QueuedUser { .. })) {
        rows.remove(index);
        return;
    }
    if matches!(rows.get(index + 1), Some(ChatItem::Assistant { text, .. }) if text.is_empty()) {
        rows.drain(index..=index + 1);
    }
}

pub(crate) fn mark_optimistic_send_failed(
    rows: &mut Vec<ChatItem>,
    display_message: &str,
    error: &str,
) {
    let Some(index) = rows.iter().rposition(|item| {
        matches!(item, ChatItem::User(value) if value == display_message)
            || matches!(item, ChatItem::QueuedUser { text, .. } if text == display_message)
    }) else {
        return;
    };
    if matches!(rows.get(index), Some(ChatItem::QueuedUser { .. })) {
        rows[index] = ChatItem::User(display_message.to_string());
        rows.insert(
            index + 1,
            ChatItem::Assistant {
                text: format!("Error: {error}"),
                model: None,
                resources: Vec::new(),
            },
        );
        return;
    }
    if let Some(ChatItem::Assistant { text, .. }) = rows.get_mut(index + 1) {
        if text.is_empty() {
            *text = format!("Error: {error}");
        }
    }
}

pub(crate) fn split_turn_started_error(error: &str) -> (bool, &str) {
    error
        .strip_prefix("[turn-started] ")
        .map_or((false, error), |message| (true, message))
}
