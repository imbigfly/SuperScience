//! TCTOKEN ability-card usage reporting and local frame binding.

use crate::app_state::AppState;
use crate::tctoken::{self, load_profile};
use tauri::State;

fn local_usage_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

async fn logged_in_user_id(store: &superscience_store::Store) -> Option<i64> {
    let profile = load_profile(store).await?;
    (profile.user_id > 0).then_some(profile.user_id)
}

/// Flat camelCase args (Tauri default): `{ cardId, cardName, date? }`.
#[tauri::command(rename = "report_ability_card_usage")]
pub async fn report_ability_card_usage_cmd(
    state: State<'_, AppState>,
    card_id: String,
    card_name: String,
    date: Option<String>,
) -> Result<(), String> {
    let card_id = card_id.trim();
    let card_name = card_name.trim();
    if card_id.is_empty() || card_name.is_empty() {
        return Ok(());
    }
    let Some(user_id) = logged_in_user_id(&state.store).await else {
        return Ok(());
    };
    if let Err(error) = tctoken::report_ability_card_usage(
        user_id,
        card_id,
        card_name,
        date.as_deref(),
    )
    .await
    {
        tracing::warn!(%error, card_id, "ability card usage report failed");
    }
    Ok(())
}

/// Flat camelCase args: `{ frameId, cardId, cardName }`.
#[tauri::command(rename = "set_frame_ability_card")]
pub async fn set_frame_ability_card_cmd(
    state: State<'_, AppState>,
    frame_id: String,
    card_id: String,
    card_name: String,
) -> Result<(), String> {
    let frame_id = frame_id.trim();
    let card_id = card_id.trim();
    let card_name = card_name.trim();
    if frame_id.is_empty() || card_id.is_empty() || card_name.is_empty() {
        return Ok(());
    }
    state
        .store
        .set_frame_ability_card(
            frame_id,
            card_id,
            card_name,
            chrono::Utc::now().timestamp(),
        )
        .await
        .map_err(|e| e.to_string())
}

/// Flat camelCase args: `{ frameId }`.
#[tauri::command(rename = "maybe_report_ability_card_resume")]
pub async fn maybe_report_ability_card_resume_cmd(
    state: State<'_, AppState>,
    frame_id: String,
) -> Result<(), String> {
    let frame_id = frame_id.trim();
    if frame_id.is_empty() {
        return Ok(());
    }
    let Some(user_id) = logged_in_user_id(&state.store).await else {
        return Ok(());
    };
    let Some((card_id, card_name)) = state
        .store
        .frame_ability_card(frame_id)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    let usage_date = local_usage_date();
    let now = chrono::Utc::now().timestamp();
    if !state
        .store
        .mark_ability_card_resume_reported(frame_id, &usage_date, now)
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(());
    }
    if let Err(error) =
        tctoken::report_ability_card_usage(user_id, &card_id, &card_name, None).await
    {
        tracing::warn!(%error, frame_id, card_id, "ability card resume usage report failed");
    }
    Ok(())
}
