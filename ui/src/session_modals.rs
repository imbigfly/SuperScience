use crate::app_support::{
    compose_icon, FileEntryModal, FolderModal, SessionTransfer, SessionTransferMode,
};
use crate::bindings::invoke_checked;
use crate::dto::*;
use crate::i18n::{t, tf, Locale};
use crate::text::{dom_value, event_target_checked, event_target_value, parent_path};
use leptos::*;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy)]
pub(crate) struct SessionTransferOverlayState {
    pub(crate) locale: RwSignal<Locale>,
    pub(crate) session_transfer: RwSignal<Option<SessionTransfer>>,
    pub(crate) session_transfer_busy: RwSignal<bool>,
    pub(crate) session_transfer_error: RwSignal<Option<String>>,
    pub(crate) project_info: RwSignal<Option<ProjectInfo>>,
    pub(crate) proj_list: RwSignal<Vec<ProjectSummary>>,
}

#[component]
pub(crate) fn SessionTransferOverlay(
    state: SessionTransferOverlayState,
    on_save: Callback<web_sys::MouseEvent>,
) -> impl IntoView {
    let SessionTransferOverlayState {
        locale,
        session_transfer,
        session_transfer_busy,
        session_transfer_error,
        project_info,
        proj_list,
    } = state;
    view! {
        {move || session_transfer.get().map(|transfer| {
            let active_project_id = project_info
                .get()
                .map(|project| project.id)
                .unwrap_or_default();
            let targets = proj_list
                .get()
                .into_iter()
                .filter(|project| project.id != active_project_id)
                .collect::<Vec<_>>();
            let has_target = !targets.is_empty() && !transfer.target_project_id.is_empty();
            let target_project_id = transfer.target_project_id.clone();
            let title_key = if transfer.mode == SessionTransferMode::Copy {
                "session.copy_title"
            } else {
                "session.move_title"
            };
            let action_key = if transfer.mode == SessionTransferMode::Copy {
                "session.copy_action"
            } else {
                "session.move_action"
            };
            view! {
            <div class="overlay">
                <div class="modal session-transfer-modal">
                    <h2>{move || t(locale.get(), title_key)}</h2>
                    <div class="hint">{tf(locale.get(), "session.transfer_hint", &[("title", &transfer.title)])}</div>
                    <label>
                        {move || t(locale.get(), "session.target_project")}
                        <select
                            disabled=move || session_transfer_busy.get()
                            on:change=move |ev| {
                                let value = event_target_value(&ev);
                                session_transfer.update(|transfer| {
                                    if let Some(transfer) = transfer {
                                        transfer.target_project_id = value;
                                    }
                                });
                            }>
                            {targets.into_iter().map(|project| {
                                let selected = project.id == target_project_id;
                                view! {
                                    <option value=project.id prop:selected=selected>{project.name}</option>
                                }
                            }).collect_view()}
                        </select>
                    </label>
                    {(!has_target).then(|| view! {
                        <div class="hint session-transfer-error">{move || t(locale.get(), "session.no_target_project")}</div>
                    })}
                    {move || session_transfer_error.get().map(|error| view! {
                        <div class="hint session-transfer-error">{error}</div>
                    })}
                    <div class="row">
                        <button type="button"
                            disabled=move || session_transfer_busy.get()
                            on:click=move |_| {
                                session_transfer.set(None);
                                session_transfer_error.set(None);
                            }>{move || t(locale.get(), "settings.cancel")}</button>
                        <button type="button" class="primary"
                            disabled=move || !has_target || session_transfer_busy.get()
                            on:click=move |ev| on_save.call(ev)>{move || t(locale.get(), action_key)}</button>
                    </div>
                </div>
            </div>
        }.into_view()
        })}
    }
}

#[derive(Clone, Copy)]
pub(crate) struct RenameSessionOverlayState {
    pub(crate) locale: RwSignal<Locale>,
    pub(crate) rename_session_target: RwSignal<Option<(String, String)>>,
    pub(crate) rename_session_input: RwSignal<String>,
}

#[component]
pub(crate) fn RenameSessionOverlay(
    state: RenameSessionOverlayState,
    on_renamed: Callback<()>,
) -> impl IntoView {
    let RenameSessionOverlayState {
        locale,
        rename_session_target,
        rename_session_input,
    } = state;
    view! {
        {move || rename_session_target.get().map(|(id, _)| {
            let id_key = id.clone();
            let id_btn = id.clone();
            view! {
            <div class="overlay">
                <div class="modal">
                    <h2>{move || t(locale.get(), "session.rename_title")}</h2>
                    <label>
                        <input
                            id="rename-session-input"
                            type="text"
                            autofocus=true
                            prop:value=move || rename_session_input.get()
                            on:input=move |ev| rename_session_input.set(dom_value(&ev))
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if (ev.ctrl_key() || ev.meta_key())
                                    && ev.key().eq_ignore_ascii_case("a")
                                {
                                    ev.prevent_default();
                                    if let Some(target) = ev.target() {
                                        if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
                                            input.select();
                                        }
                                    }
                                    return;
                                }
                                if ev.key() == "Enter" {
                                    ev.prevent_default();
                                    let title = rename_session_input.get().trim().to_string();
                                    if title.is_empty() { return; }
                                    let id = id_key.clone();
                                    rename_session_target.set(None);
                                    spawn_local(async move {
                                        let arg = to_value(&serde_json::json!({ "id": id, "title": title })).unwrap();
                                        if invoke_checked("rename_session", arg).await.is_ok() {
                                            on_renamed.call(());
                                        }
                                    });
                                }
                            }
                        />
                    </label>
                    <div class="row">
                        <button on:click=move |_| rename_session_target.set(None)>{move || t(locale.get(), "settings.cancel")}</button>
                        <button class="primary" on:click=move |_| {
                            let title = rename_session_input.get().trim().to_string();
                            if title.is_empty() { return; }
                            let id = id_btn.clone();
                            rename_session_target.set(None);
                            spawn_local(async move {
                                let arg = to_value(&serde_json::json!({ "id": id, "title": title })).unwrap();
                                if invoke_checked("rename_session", arg).await.is_ok() {
                                    on_renamed.call(());
                                }
                            });
                        }>{move || t(locale.get(), "settings.save")}</button>
                    </div>
                </div>
            </div>
        }.into_view()
        })}
    }
}

#[derive(Clone, Copy)]
pub(crate) struct FolderModalOverlayState {
    pub(crate) locale: RwSignal<Locale>,
    pub(crate) folder_modal: RwSignal<Option<FolderModal>>,
    pub(crate) folder_modal_input: RwSignal<String>,
}

#[component]
pub(crate) fn FolderModalOverlay(
    state: FolderModalOverlayState,
    on_save: Callback<FolderModal>,
) -> impl IntoView {
    let FolderModalOverlayState {
        locale,
        folder_modal,
        folder_modal_input,
    } = state;
    view! {
        {move || folder_modal.get().map(|mode| {
            let mode_save = mode.clone();
            let mode_enter = mode.clone();
            let title_key = match &mode {
                FolderModal::Create => "folder.new_title",
                FolderModal::Rename(_) => "folder.rename_prompt",
            };
            let label_key = match &mode {
                FolderModal::Create => "folder.new_prompt",
                FolderModal::Rename(_) => "folder.new_prompt",
            };
            view! {
            <div class="overlay">
                <div class="modal">
                    <h2>{move || t(locale.get(), title_key)}</h2>
                    <label>
                        {move || t(locale.get(), label_key)}
                        <input
                            id="folder-modal-input"
                            type="text"
                            autofocus=true
                            prop:value=move || folder_modal_input.get()
                            on:input=move |ev| folder_modal_input.set(dom_value(&ev))
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if ev.key() == "Enter" {
                                    ev.prevent_default();
                                    on_save.call(mode_enter.clone());
                                }
                            }
                        />
                    </label>
                    <div class="row">
                        <button on:click=move |_| folder_modal.set(None)>{move || t(locale.get(), "settings.cancel")}</button>
                        <button class="primary" on:click=move |_| on_save.call(mode_save.clone())>{move || t(locale.get(), "settings.save")}</button>
                    </div>
                </div>
            </div>
        }.into_view()
        })}
    }
}

#[derive(Clone, Copy)]
pub(crate) struct FileEntryOverlayState {
    pub(crate) locale: RwSignal<Locale>,
    pub(crate) file_entry_modal: RwSignal<Option<FileEntryModal>>,
    pub(crate) file_entry_input: RwSignal<String>,
    pub(crate) file_entry_busy: RwSignal<bool>,
    pub(crate) file_entry_error: RwSignal<Option<String>>,
    pub(crate) file_cwd: RwSignal<String>,
}

#[component]
pub(crate) fn FileEntryOverlay(
    state: FileEntryOverlayState,
    on_save: Callback<FileEntryModal>,
) -> impl IntoView {
    let FileEntryOverlayState {
        locale,
        file_entry_modal,
        file_entry_input,
        file_entry_busy,
        file_entry_error,
        file_cwd,
    } = state;
    view! {
        {move || file_entry_modal.get().map(|mode| {
            let mode_save = mode.clone();
            let mode_enter = mode.clone();
            let (title_key, action_key, location) = match &mode {
                FileEntryModal::CreateFile => (
                    "files.new_file",
                    "files.create",
                    file_cwd.get_untracked(),
                ),
                FileEntryModal::CreateDirectory => (
                    "files.new_directory",
                    "files.create",
                    file_cwd.get_untracked(),
                ),
                FileEntryModal::Rename { path, is_dir } => (
                    if *is_dir { "files.rename_directory" } else { "files.rename_file" },
                    "files.rename",
                    parent_path(path),
                ),
            };
            view! {
                <div class="overlay">
                    <div class="modal file-entry-modal">
                        <h2>{move || t(locale.get(), title_key)}</h2>
                        <div class="hint file-entry-location">
                            {move || tf(locale.get(), "files.location", &[("path", &location)])}
                        </div>
                        <label>
                            {move || t(locale.get(), "files.name")}
                            <input
                                id="file-entry-modal-input"
                                type="text"
                                autofocus=true
                                disabled=move || file_entry_busy.get()
                                prop:value=move || file_entry_input.get()
                                on:input=move |ev| {
                                    file_entry_input.set(dom_value(&ev));
                                    file_entry_error.set(None);
                                }
                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                    if ev.key() == "Enter" {
                                        ev.prevent_default();
                                        on_save.call(mode_enter.clone());
                                    }
                                }
                            />
                        </label>
                        {move || file_entry_error.get().map(|error| view! {
                            <div class="settings-error" role="alert">{error}</div>
                        })}
                        <div class="row">
                            <button disabled=move || file_entry_busy.get() on:click=move |_| {
                                file_entry_modal.set(None);
                                file_entry_error.set(None);
                            }>{move || t(locale.get(), "settings.cancel")}</button>
                            <button class="primary" disabled=move || file_entry_busy.get()
                                on:click=move |_| on_save.call(mode_save.clone())>
                                {move || if file_entry_busy.get() {
                                    t(locale.get(), "files.working")
                                } else {
                                    t(locale.get(), action_key)
                                }}
                            </button>
                        </div>
                    </div>
                </div>
            }.into_view()
        })}
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TurnUndoOverlayState {
    pub(crate) locale: RwSignal<Locale>,
    pub(crate) turn_undo_dialog: RwSignal<Option<TurnUndoDialog>>,
    pub(crate) turn_undo_busy: RwSignal<bool>,
    pub(crate) turn_undo_error: RwSignal<Option<String>>,
}

#[component]
pub(crate) fn TurnUndoOverlay(
    state: TurnUndoOverlayState,
    on_confirm: Callback<()>,
) -> impl IntoView {
    let TurnUndoOverlayState {
        locale,
        turn_undo_dialog,
        turn_undo_busy,
        turn_undo_error,
    } = state;
    view! {
        {move || turn_undo_dialog.get().map(|dialog| {
            let restore_files = dialog.preview.restore_files.clone();
            let remove_files = dialog.preview.remove_files.clone();
            let remove_artifacts = dialog.preview.remove_artifacts.clone();
            let unsupported_files = dialog.preview.unsupported_files.clone();
            let conflicts = dialog.preview.conflicts.clone();
            let has_text_changes = !restore_files.is_empty() || !remove_files.is_empty();
            let can_confirm = conflicts.is_empty();
            view! {
                <div class="overlay">
                    <div
                        class="modal confirm-modal turn-undo-modal"
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby="turn-undo-title"
                        data-testid="turn-undo-modal"
                    >
                        <h2 id="turn-undo-title">{move || t(locale.get(), "undo.title")}</h2>
                        <div class="turn-undo-scroll">
                            <p class="turn-undo-body">{move || t(locale.get(), "undo.body")}</p>
                            <div class="turn-undo-warning">
                                {move || t(locale.get(), "undo.binary_warning")}
                            </div>
                            {(!has_text_changes).then(|| view! {
                                <p class="turn-undo-empty">
                                    {move || t(locale.get(), "undo.no_text_changes")}
                                </p>
                            })}
                            {(!restore_files.is_empty()).then(|| view! {
                                <section class="turn-undo-section">
                                    <h3>{move || t(locale.get(), "undo.restore_files")}</h3>
                                    <ul>{restore_files.into_iter().map(|path| view! {
                                        <li><code>{path}</code></li>
                                    }).collect_view()}</ul>
                                </section>
                            })}
                            {(!remove_files.is_empty()).then(|| view! {
                                <section class="turn-undo-section">
                                    <h3>{move || t(locale.get(), "undo.remove_files")}</h3>
                                    <ul>{remove_files.into_iter().map(|path| view! {
                                        <li><code>{path}</code></li>
                                    }).collect_view()}</ul>
                                </section>
                            })}
                            {(!remove_artifacts.is_empty()).then(|| view! {
                                <section class="turn-undo-section">
                                    <h3>{move || t(locale.get(), "undo.remove_artifacts")}</h3>
                                    <ul>{remove_artifacts.into_iter().map(|name| view! {
                                        <li><code>{name}</code></li>
                                    }).collect_view()}</ul>
                                </section>
                            })}
                            {(!unsupported_files.is_empty()).then(|| view! {
                                <section class="turn-undo-section unsupported">
                                    <h3>{move || t(locale.get(), "undo.unsupported_files")}</h3>
                                    <ul>{unsupported_files.into_iter().map(|path| view! {
                                        <li><code>{path}</code></li>
                                    }).collect_view()}</ul>
                                </section>
                            })}
                            {(!conflicts.is_empty()).then(|| view! {
                                <section class="turn-undo-section conflicts">
                                    <h3>{move || t(locale.get(), "undo.conflicts")}</h3>
                                    <ul>{conflicts.into_iter().map(|path| view! {
                                        <li><code>{path}</code></li>
                                    }).collect_view()}</ul>
                                </section>
                            })}
                            {move || turn_undo_error.get().map(|error| view! {
                                <div class="turn-undo-error" role="alert">{error}</div>
                            })}
                        </div>
                        <div class="row">
                            <button
                                disabled=move || turn_undo_busy.get()
                                on:click=move |_| {
                                    turn_undo_dialog.set(None);
                                    turn_undo_error.set(None);
                                }
                            >
                                {move || t(locale.get(), "settings.cancel")}
                            </button>
                            <button
                                class="primary"
                                disabled=move || turn_undo_busy.get() || !can_confirm
                                on:click=move |_| on_confirm.call(())
                            >
                                {move || if turn_undo_busy.get() {
                                    t(locale.get(), "undo.working")
                                } else {
                                    t(locale.get(), "undo.confirm")
                                }}
                            </button>
                        </div>
                    </div>
                </div>
            }.into_view()
        })}
    }
}

#[derive(Clone, Copy)]
pub(crate) struct EditConfirmOverlayState {
    pub(crate) locale: RwSignal<Locale>,
    pub(crate) edit_confirm: RwSignal<Option<usize>>,
}

#[component]
pub(crate) fn EditConfirmOverlay(
    state: EditConfirmOverlayState,
    on_branch: Callback<usize>,
    on_rewind: Callback<usize>,
) -> impl IntoView {
    let EditConfirmOverlayState {
        locale,
        edit_confirm,
    } = state;
    view! {
        {move || edit_confirm.get().map(|ui_index| {
            view! {
                <div class="overlay">
                    <div
                        class="modal confirm-modal"
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby="edit-confirm-title"
                        data-testid="edit-confirm-modal"
                    >
                        <h2 id="edit-confirm-title">{move || t(locale.get(), "msg.edit_confirm_title")}</h2>
                        <div class="hint">{move || t(locale.get(), "msg.edit_confirm_hint")}</div>
                        <div class="row">
                            <button on:click=move |_| edit_confirm.set(None)>
                                {move || t(locale.get(), "settings.cancel")}
                            </button>
                            <button on:click=move |_| {
                                edit_confirm.set(None);
                                on_branch.call(ui_index);
                            }>
                                {move || t(locale.get(), "msg.branch")}
                            </button>
                            <button class="primary" class:danger=true on:click=move |_| {
                                edit_confirm.set(None);
                                on_rewind.call(ui_index);
                            }>
                                {move || t(locale.get(), "msg.edit")}
                            </button>
                        </div>
                    </div>
                </div>
            }
        })}
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ModelSwitchConfirmOverlayState {
    pub(crate) locale: RwSignal<Locale>,
    pub(crate) model_switch_confirm: RwSignal<Option<(String, String, bool)>>,
}

#[component]
pub(crate) fn ModelSwitchConfirmOverlay(
    state: ModelSwitchConfirmOverlayState,
    on_switch: Callback<(String, bool)>,
) -> impl IntoView {
    let ModelSwitchConfirmOverlayState {
        locale,
        model_switch_confirm,
    } = state;
    view! {
        {move || model_switch_confirm.get().map(|(id, label, ignores_images)| {
            let switch_yes = on_switch.clone();
            let yes_id = id.clone();
            let dont_ask_again = create_rw_signal(false);
            let hint_key = if ignores_images {
                "models.switch_confirm_image_hint"
            } else {
                "models.switch_confirm_hint"
            };
            let yes_key = if ignores_images {
                "models.switch_ignore_images"
            } else {
                "models.switch_yes"
            };
            view! {
                <div class="overlay" data-testid="model-switch-confirm-overlay">
                    <div class="modal confirm-modal model-switch-confirm" data-testid="model-switch-confirm">
                        <h2>{move || t(locale.get(), "models.switch_confirm_title")}</h2>
                        <div class="hint">{move || tf(
                            locale.get(),
                            hint_key,
                            &[("model", &label)],
                        )}</div>
                        <label class="confirm-option" data-testid="model-switch-dont-ask">
                            <input type="checkbox"
                                prop:checked=move || dont_ask_again.get()
                                on:change=move |ev| dont_ask_again.set(event_target_checked(&ev)) />
                            <span>{move || t(locale.get(), "models.switch_dont_ask")}</span>
                        </label>
                        <div class="row">
                            <button type="button" on:click=move |_| model_switch_confirm.set(None)>
                                {move || t(locale.get(), "models.switch_no")}
                            </button>
                            <button type="button" class="primary" on:click=move |_| {
                                let skip_future = dont_ask_again.get_untracked();
                                model_switch_confirm.set(None);
                                switch_yes.call((yes_id.clone(), skip_future));
                            }>{move || t(locale.get(), yes_key)}</button>
                        </div>
                    </div>
                </div>
            }.into_view()
        })}
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ProjSettingsOverlayState {
    pub(crate) locale: RwSignal<Locale>,
    pub(crate) show_proj_settings: RwSignal<bool>,
    pub(crate) proj_settings: RwSignal<ProjectSettings>,
    pub(crate) proj_settings_busy: RwSignal<bool>,
}

#[component]
pub(crate) fn ProjSettingsOverlay(
    state: ProjSettingsOverlayState,
    on_save: Callback<web_sys::MouseEvent>,
) -> impl IntoView {
    let ProjSettingsOverlayState {
        locale,
        show_proj_settings,
        proj_settings,
        proj_settings_busy,
    } = state;
    view! {
        {move || show_proj_settings.get().then(|| view! {
            <div class="overlay">
                <div class="modal proj-settings-modal">
                    <div class="ps-head">
                        <h2>{move || t(locale.get(), "proj_settings.title")}</h2>
                        <button type="button" class="ps-close"
                            title=move || t(locale.get(), "settings.cancel")
                            on:click=move |_| show_proj_settings.set(false)>{compose_icon("close")}</button>
                    </div>
                    <label>
                        {move || t(locale.get(), "proj_settings.name")}
                        <input prop:value=move || proj_settings.get().name
                            on:input=move |ev| { let v = event_target_value(&ev); proj_settings.update(|s| s.name = v); } />
                    </label>
                    <label>
                        {move || t(locale.get(), "proj_settings.description")}
                        <span class="ps-hint">{move || t(locale.get(), "proj_settings.description_hint")}</span>
                        <textarea class="ps-textarea" rows="2"
                            prop:value=move || proj_settings.get().description
                            on:input=move |ev| { let v = event_target_value(&ev); proj_settings.update(|s| s.description = v); }></textarea>
                    </label>
                    <label>
                        {move || t(locale.get(), "proj_settings.agent_context")}
                        <span class="ps-hint">{move || t(locale.get(), "proj_settings.agent_context_hint")}</span>
                        <textarea class="ps-textarea ps-ctx" rows="8"
                            prop:value=move || proj_settings.get().agent_context
                            on:input=move |ev| { let v = event_target_value(&ev); proj_settings.update(|s| s.agent_context = v); }></textarea>
                    </label>
                    <div class="row">
                        <button type="button" disabled=move || proj_settings_busy.get()
                            on:click=move |_| show_proj_settings.set(false)>{move || t(locale.get(), "settings.cancel")}</button>
                        <button type="button" class="primary"
                            disabled=move || proj_settings_busy.get() || proj_settings.get().name.trim().is_empty()
                            on:click=move |ev| on_save.call(ev)>{move || t(locale.get(), "settings.save")}</button>
                    </div>
                </div>
            </div>
        })}
    }
}
