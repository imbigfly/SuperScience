//! Runtime Commands split out of lib.rs; shared state/helpers stay in the crate root.

use super::*;

#[tauri::command]
pub(super) async fn list_execution_contexts(
    state: State<'_, AppState>,
) -> Result<Vec<wisp_store::ExecutionContext>, String> {
    state
        .store
        .list_execution_contexts()
        .await
        .map_err(|e| format!("{e}"))
}

#[tauri::command]
pub(super) async fn list_runtimes(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<Vec<wisp_runtime::RuntimeInfo>, String> {
    let (_, scope) =
        exploration_commands::working_project_for_active_frame(&state, window.label()).await?;
    Ok(state
        .runtime_manager
        .list()
        .into_iter()
        .filter(|runtime| match &scope {
            wisp_store::StateScope::Mainline { .. } => {
                runtime.key.scope_key == wisp_runtime::MAINLINE_RUNTIME_SCOPE
            }
            wisp_store::StateScope::Exploration {
                project_id,
                exploration_id,
            } => runtime.key.project_id == *project_id && runtime.key.scope_key == *exploration_id,
        })
        .collect())
}

#[tauri::command]
pub(super) async fn inspect_runtime(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    project_id: String,
    context_id: String,
    language: wisp_runtime::RuntimeLanguage,
) -> Result<wisp_runtime::RuntimeObjectList, String> {
    let (_, scope) =
        exploration_commands::working_project_for_active_frame(&state, window.label()).await?;
    if matches!(&scope, wisp_store::StateScope::Exploration { .. })
        && scope.project_id() != project_id
    {
        return Err(
            "exploration_scope_violation: cross-project runtime inspection is disabled".into(),
        );
    }
    let scope_key = if scope.project_id() == project_id {
        scope.scope_key()
    } else {
        wisp_runtime::MAINLINE_RUNTIME_SCOPE
    };
    state
        .runtime_manager
        .inspect(&wisp_runtime::RuntimeKey {
            project_id,
            scope_key: scope_key.into(),
            context_id,
            language,
        })
        .await
        .map_err(|error| error.to_string())
}

/// Run code the user selected in the file preview against their bound runtime.
/// Deferred in the runtime design until the UI gained a code editor; it has one
/// now. The user is looking at the code they pressed Run on, so this path is
/// deliberately outside the agent tool-approval flow.
///
/// Returns console text. Code that raised is still `Ok`: `format_response` tags
/// it `[error]` exactly as the agent tools render it. `Err` means the runtime
/// itself never produced a result.
#[tauri::command]
pub(super) async fn execute_runtime(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    context_id: String,
    language: wisp_runtime::RuntimeLanguage,
    code: String,
) -> Result<String, String> {
    if code.len() > wisp_runtime::MAX_CODE_BYTES {
        return Err(format!(
            "Selection exceeds the {} byte runtime limit.",
            wisp_runtime::MAX_CODE_BYTES
        ));
    }
    let (project, scope) =
        exploration_commands::working_project_for_active_frame(&state, window.label()).await?;
    let _project_activity = state.begin_project_activity(&project.id)?;
    exploration_commands::require_writable_scope(&state.store, &scope).await?;
    let key = wisp_runtime::RuntimeKey {
        project_id: project.id.clone(),
        scope_key: scope.scope_key().to_string(),
        context_id,
        language,
    };
    let mut execution = state
        .runtime_manager
        .execute(&key, &project.root, code)
        .await
        .map_err(|error| error.to_string())?;
    loop {
        match execution.recv().await {
            // ponytail: buffered, not streamed — the final frame repeats every
            // chunk. Stream to the console when a cell runs long enough to care.
            Some(wisp_runtime::RuntimeEvent::Stdout(_)) => {}
            Some(wisp_runtime::RuntimeEvent::Finished(result)) => {
                state
                    .store
                    .bump_state_generation(&scope)
                    .await
                    .map_err(|error| error.to_string())?;
                return result
                    .map(|response| wisp_runtime::format_response(&response))
                    .map_err(|error| error.to_string());
            }
            None => return Err("Runtime ended before returning a result.".into()),
        }
    }
}

#[tauri::command]
pub(super) async fn start_runtime(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    context_id: String,
    language: wisp_runtime::RuntimeLanguage,
) -> Result<wisp_runtime::RuntimeInfo, String> {
    let (project, scope) =
        exploration_commands::working_project_for_active_frame(&state, window.label()).await?;
    let _project_activity = state.begin_project_activity(&project.id)?;
    exploration_commands::require_writable_scope(&state.store, &scope).await?;
    state
        .runtime_manager
        .start(
            wisp_runtime::RuntimeKey {
                project_id: project.id.clone(),
                scope_key: scope.scope_key().to_string(),
                context_id,
                language,
            },
            project.root,
        )
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(super) async fn stop_runtime(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    project_id: String,
    context_id: String,
    language: wisp_runtime::RuntimeLanguage,
) -> Result<Option<wisp_runtime::RuntimeInfo>, String> {
    let (_, scope) =
        exploration_commands::working_project_for_active_frame(&state, window.label()).await?;
    if matches!(&scope, wisp_store::StateScope::Exploration { .. })
        && scope.project_id() != project_id
    {
        return Err(
            "exploration_scope_violation: cross-project runtime control is disabled".into(),
        );
    }
    let scope_key = if scope.project_id() == project_id {
        scope.scope_key()
    } else {
        wisp_runtime::MAINLINE_RUNTIME_SCOPE
    };
    Ok(state
        .runtime_manager
        .stop(&wisp_runtime::RuntimeKey {
            project_id,
            scope_key: scope_key.to_string(),
            context_id,
            language,
        })
        .await)
}

#[tauri::command]
pub(super) async fn restart_runtime(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    project_id: String,
    context_id: String,
    language: wisp_runtime::RuntimeLanguage,
) -> Result<wisp_runtime::RuntimeInfo, String> {
    let (working, scope) =
        exploration_commands::working_project_for_active_frame(&state, window.label()).await?;
    if matches!(&scope, wisp_store::StateScope::Exploration { .. })
        && scope.project_id() != project_id
    {
        return Err(
            "exploration_scope_violation: cross-project runtime control is disabled".into(),
        );
    }
    let (root, target_scope) = if working.id == project_id {
        (working.root, scope)
    } else {
        let (_, workspace) = state
            .store
            .get_project(&project_id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Project not found: {project_id}"))?;
        (
            ensure_writable(PathBuf::from(workspace), &state.app_data),
            wisp_store::StateScope::mainline(project_id.clone()),
        )
    };
    let _project_activity = state.begin_project_activity(&project_id)?;
    exploration_commands::require_writable_scope(&state.store, &target_scope).await?;
    state
        .runtime_manager
        .restart(
            wisp_runtime::RuntimeKey {
                project_id,
                scope_key: target_scope.scope_key().to_string(),
                context_id,
                language,
            },
            root,
        )
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(super) async fn list_runs(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<Vec<wisp_store::RunSummary>, String> {
    let (_, scope) =
        exploration_commands::working_project_for_active_frame(&state, window.label()).await?;
    state
        .store
        .list_run_summaries_in_scope(&scope)
        .await
        .map_err(|e| format!("{e}"))
}

#[tauri::command]
pub(super) async fn get_run_detail(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    run_id: String,
) -> Result<wisp_store::RunRecord, String> {
    let (_, scope) =
        exploration_commands::working_project_for_active_frame(&state, window.label()).await?;
    if !state
        .store
        .run_visible_in_scope(&run_id, &scope)
        .await
        .map_err(|error| error.to_string())?
    {
        return Err("Run is not visible in the active state scope".into());
    }
    state
        .store
        .get_run(&run_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Run not found".into())
}

#[tauri::command]
pub(super) async fn cancel_run(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    run_id: String,
) -> Result<wisp_store::RunRecord, String> {
    let (ap, scope) =
        exploration_commands::working_project_for_active_frame(&state, window.label()).await?;
    let run = state
        .store
        .get_run(&run_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Run not found".to_string())?;
    if run.project_id != ap.id {
        return Err("Run does not belong to the active project".into());
    }
    if state
        .store
        .run_state_scope(&run_id)
        .await
        .map_err(|error| error.to_string())?
        .as_ref()
        != Some(&scope)
    {
        return Err("Run is not visible in the active state scope".into());
    }
    let _project_activity = state.begin_project_activity(&ap.id)?;
    state.run_manager.cancel(&state.store, &run_id).await?;
    state
        .store
        .get_run(&run_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Run disappeared after cancellation".to_string())
}

/// Retry output registration/download for a succeeded Run whose declared
/// outputs were never harvested.
#[tauri::command]
pub(super) async fn harvest_run(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    run_id: String,
) -> Result<wisp_store::RunRecord, String> {
    let (ap, scope) =
        exploration_commands::working_project_for_active_frame(&state, window.label()).await?;
    let run = state
        .store
        .get_run(&run_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Run not found".to_string())?;
    if run.project_id != ap.id {
        return Err("Run does not belong to the active project".into());
    }
    if state
        .store
        .run_state_scope(&run_id)
        .await
        .map_err(|error| error.to_string())?
        .as_ref()
        != Some(&scope)
    {
        return Err("Run is not visible in the active state scope".into());
    }
    let _project_activity = state.begin_project_activity(&ap.id)?;
    state.run_manager.harvest_run(&state.store, &run_id).await?;
    state
        .store
        .get_run(&run_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Run disappeared after harvest".to_string())
}

/// Delete a terminal Run's server-side workspace. `force` carries the user's
/// explicit confirmation when unharvested outputs would be lost.
#[tauri::command]
pub(super) async fn cleanup_run_workspace(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    run_id: String,
    force: Option<bool>,
) -> Result<wisp_store::RunRecord, String> {
    let (ap, scope) =
        exploration_commands::working_project_for_active_frame(&state, window.label()).await?;
    let run = state
        .store
        .get_run(&run_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Run not found".to_string())?;
    if run.project_id != ap.id {
        return Err("Run does not belong to the active project".into());
    }
    if state
        .store
        .run_state_scope(&run_id)
        .await
        .map_err(|error| error.to_string())?
        .as_ref()
        != Some(&scope)
    {
        return Err("Run is not visible in the active state scope".into());
    }
    let _project_activity = state.begin_project_activity(&ap.id)?;
    state
        .run_manager
        .cleanup_run_workspace(&state.store, &run_id, force.unwrap_or(false))
        .await
}
