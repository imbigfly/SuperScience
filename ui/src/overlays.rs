use crate::app_support::{
    compose_icon, js_error_text, refresh_execution_contexts, refresh_runs, refresh_runtimes,
    show_toast,
};
use crate::bindings::{invoke_checked, open_external_url};
use crate::dto::*;
use crate::i18n::{localize_backend, t, tf, Locale};
use crate::text::{dom_value, event_target_value};
use leptos::*;
use serde_wasm_bindgen::to_value;

#[component]
pub(super) fn AddHostOverlay(
    locale: RwSignal<Locale>,
    show_add_host: RwSignal<bool>,
    host_alias: RwSignal<String>,
    host_hostname: RwSignal<String>,
    host_notes: RwSignal<String>,
    host_user: RwSignal<String>,
    host_port: RwSignal<String>,
    host_identity: RwSignal<String>,
    host_auth_method: RwSignal<String>,
    host_password: RwSignal<String>,
    host_has_password: RwSignal<bool>,
    editing_host_alias: RwSignal<Option<String>>,
    ssh_hosts: RwSignal<Vec<SshHost>>,
    execution_contexts: RwSignal<Vec<ExecutionContext>>,
) -> impl IntoView {
    let build_host = move || {
        let opt = |s: String| {
            let s = s.trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        };
        let auth = host_auth_method.get();
        let auth = if auth == "password" {
            "password"
        } else {
            "key"
        };
        SshHost {
            alias: host_alias.get().trim().to_string(),
            host_name: opt(host_hostname.get()),
            user: opt(host_user.get()),
            port: host_port.get().trim().parse::<u16>().ok(),
            identity_file: if auth == "key" {
                opt(host_identity.get())
            } else {
                None
            },
            notes: opt(host_notes.get()),
            auth_method: Some(auth.into()),
            has_password: false,
            password: if auth == "password" {
                opt(host_password.get())
            } else {
                None
            },
        }
    };
    let testing = create_rw_signal(false);
    let test_result = create_rw_signal::<Option<Result<(), String>>>(None);
    move || {
        show_add_host.get().then(|| view! {
    <div class="overlay">
        <div class="modal host-modal" role="dialog" aria-modal="true"
            aria-labelledby="host-modal-title">
            <div class="ps-head">
                <h2 id="host-modal-title">{move || if editing_host_alias.get().is_some() {
                    t(locale.get(), "hosts.edit")
                } else {
                    t(locale.get(), "hosts.add")
                }}</h2>
                <button type="button" class="ps-close"
                    title=move || t(locale.get(), "hosts.cancel")
                    aria-label=move || t(locale.get(), "hosts.cancel")
                    on:click=move |_| {
                        editing_host_alias.set(None);
                        test_result.set(None);
                        show_add_host.set(false);
                    }>
                    {compose_icon("close")}
                </button>
            </div>
            <label class="host-label" for="add-host-alias">{move || t(locale.get(), "hosts.name")}</label>
            <input id="add-host-alias" class="host-input" autofocus=true
                disabled=move || editing_host_alias.get().is_some()
                placeholder=move || t(locale.get(), "hosts.name_ph")
                prop:value=move || host_alias.get()
                on:input=move |ev| host_alias.set(event_target_value(&ev)) />
            <label class="host-label" for="host-hostname">{move || t(locale.get(), "hosts.hostname")}</label>
            <input id="host-hostname" class="host-input"
                placeholder=move || t(locale.get(), "hosts.hostname_ph")
                prop:value=move || host_hostname.get()
                on:input=move |ev| host_hostname.set(event_target_value(&ev)) />
            <label class="host-label" for="host-user">{move || t(locale.get(), "hosts.user")}</label>
            <input id="host-user" class="host-input" prop:value=move || host_user.get()
                placeholder=move || t(locale.get(), "hosts.user_ph")
                on:input=move |ev| host_user.set(event_target_value(&ev)) />
            <label class="host-label" for="host-port">{move || t(locale.get(), "hosts.port")}</label>
            <input id="host-port" class="host-input" placeholder="22" prop:value=move || host_port.get()
                on:input=move |ev| host_port.set(event_target_value(&ev)) />
            <label class="host-label">{move || t(locale.get(), "hosts.auth_method")}</label>
            <select class="host-input" prop:value=move || host_auth_method.get()
                on:change=move |ev| host_auth_method.set(dom_value(&ev))>
                <option value="key">{move || t(locale.get(), "hosts.auth_key")}</option>
                <option value="password">{move || t(locale.get(), "hosts.auth_password")}</option>
            </select>
            {move || if host_auth_method.get() == "password" {
                view! {
                    <label class="host-label" for="host-password">{t(locale.get(), "hosts.password")}</label>
                    <input id="host-password" class="host-input" type="password" autocomplete="new-password"
                        prop:value=move || host_password.get()
                        placeholder=move || if host_has_password.get() {
                            t(locale.get(), "hosts.password_keep").to_string()
                        } else {
                            t(locale.get(), "hosts.password_ph").to_string()
                        }
                        on:input=move |ev| host_password.set(event_target_value(&ev)) />
                    <p class="hint">{t(locale.get(), "hosts.password_hint")}</p>
                }.into_view()
            } else {
                view! {
                    <label class="host-label" for="host-identity">{t(locale.get(), "hosts.identity")}</label>
                    <input id="host-identity" class="host-input" prop:value=move || host_identity.get()
                        placeholder=move || t(locale.get(), "hosts.identity_ph")
                        on:input=move |ev| host_identity.set(event_target_value(&ev)) />
                }.into_view()
            }}
            <label class="host-label" for="host-notes">{move || t(locale.get(), "hosts.notes")}</label>
            <textarea id="host-notes" class="host-input" prop:value=move || host_notes.get()
                placeholder=move || t(locale.get(), "hosts.notes_ph")
                on:input=move |ev| host_notes.set(event_target_value(&ev))></textarea>
            {move || test_result.get().map(|result| match result {
                Ok(()) => view! { <p class="settings-status ok">{t(locale.get(), "hosts.test_ok")}</p> },
                Err(error) => view! { <p class="settings-status fail">{localize_backend(locale.get(), &error)}</p> },
            })}
            <div class="row">
                <button type="button"
                    disabled=move || testing.get() || host_alias.get().trim().is_empty()
                    on:click=move |_| {
                        let host = build_host();
                        testing.set(true);
                        test_result.set(None);
                        let arg = to_value(&serde_json::json!({ "host": host })).unwrap();
                        spawn_local(async move {
                            let result = invoke_checked("test_ssh_connection", arg).await;
                            test_result.set(Some(result.map(|_| ()).map_err(|error| js_error_text(error))));
                            testing.set(false);
                        });
                    }>{move || if testing.get() {
                        t(locale.get(), "hosts.testing")
                    } else {
                        t(locale.get(), "hosts.test")
                    }}</button>
                <button type="button" on:click=move |_| {
                    editing_host_alias.set(None);
                    test_result.set(None);
                    show_add_host.set(false);
                }>{move || t(locale.get(), "hosts.cancel")}</button>
                <button type="button" class="primary" disabled=move || {
                    let alias_empty = host_alias.get().trim().is_empty();
                    let password_missing = host_auth_method.get() == "password"
                        && host_password.get().trim().is_empty()
                        && !host_has_password.get();
                    alias_empty || password_missing
                }
                    on:click=move |_| {
                        let host = build_host();
                        let arg = to_value(&serde_json::json!({ "host": host })).unwrap();
                        spawn_local(async move {
                            match invoke_checked("add_ssh_host", arg).await {
                                Ok(v) => {
                                    if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<SshHost>>(v) {
                                        ssh_hosts.set(list);
                                        refresh_execution_contexts(execution_contexts);
                                    }
                                }
                                Err(error) => {
                                    show_toast(&localize_backend(locale.get_untracked(), &js_error_text(error)));
                                }
                            }
                        });
                        host_alias.set(String::new()); host_hostname.set(String::new()); host_user.set(String::new()); host_port.set(String::new());
                        host_identity.set(String::new()); host_notes.set(String::new());
                        host_auth_method.set("key".into()); host_password.set(String::new());
                        host_has_password.set(false);
                        editing_host_alias.set(None);
                        test_result.set(None);
                        show_add_host.set(false);
                    }>{move || if editing_host_alias.get().is_some() {
                        t(locale.get(), "hosts.update")
                    } else {
                        t(locale.get(), "hosts.save")
                    }}</button>
            </div>
        </div>
    </div>
}.into_view())
    }
}

#[component]
pub(super) fn RuntimeInterpreterOverlay(
    locale: RwSignal<Locale>,
    form: RwSignal<Option<RuntimeInterpreterForm>>,
    execution_contexts: RwSignal<Vec<ExecutionContext>>,
    runtimes: RwSignal<Vec<RuntimeInfo>>,
) -> impl IntoView {
    let busy = create_rw_signal(false);
    let error = create_rw_signal(None::<String>);
    // Render the dialog from its open state, not from the whole editable form.
    // Otherwise each input event (including paste) replaces the modal DOM and
    // drops focus.
    let open = create_memo(move |_| form.with(|value| value.is_some()));
    // Native file picker for local interpreters; remote contexts keep manual
    // entry because the path must exist on the remote host (#651).
    let browse = move |field: &'static str| {
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let args = to_value(&serde_json::json!({})).unwrap();
            match invoke_checked("pick_executable_file", args).await {
                Ok(value) => {
                    if let Some(path) = value.as_string().filter(|path| !path.is_empty()) {
                        form.update(|current| {
                            if let Some(current) = current {
                                match field {
                                    "python" => current.python_executable = path.clone(),
                                    _ => current.rscript_executable = path.clone(),
                                }
                            }
                        });
                    }
                }
                Err(value) => error.set(Some(localize_backend(
                    locale.get_untracked(),
                    &js_error_text(value),
                ))),
            }
            busy.set(false);
        });
    };
    let save = move |_| {
        let Some(current) = form.get_untracked() else {
            return;
        };
        busy.set(true);
        error.set(None);
        let args = to_value(&serde_json::json!({
            "contextId": current.context_id,
            "pythonExecutable": current.python_executable,
            "rscriptExecutable": current.rscript_executable,
        }))
        .unwrap();
        spawn_local(async move {
            match invoke_checked("update_execution_context_interpreters", args).await {
                Ok(_) => {
                    refresh_execution_contexts(execution_contexts);
                    refresh_runtimes(runtimes);
                    form.set(None);
                    show_toast(&t(locale.get_untracked(), "runtime_config.saved"));
                }
                Err(value) => error.set(Some(localize_backend(
                    locale.get_untracked(),
                    &js_error_text(value),
                ))),
            }
            busy.set(false);
        });
    };

    move || {
        open.get().then(|| view! {
                <div class="overlay">
                    <div class="modal runtime-config-modal">
                        <div class="ps-head">
                            <h2>{move || t(locale.get(), "runtime_config.title")}</h2>
                            <button type="button" class="ps-close"
                                title=move || t(locale.get(), "settings.cancel")
                                disabled=move || busy.get()
                                on:click=move |_| form.set(None)>{compose_icon("close")}</button>
                        </div>
                        <p class="runtime-config-hint">{
                            move || {
                                let context = form.with(|value| value.as_ref()
                                    .map(|value| value.context_label.clone())
                                    .unwrap_or_default());
                                tf(locale.get(), "runtime_config.scope", &[("context", &context)])
                            }
                        }</p>
                        <label>
                            {move || t(locale.get(), "runtime_config.python")}
                            <div class="runtime-config-picker">
                                <input id="runtime-python-executable" autocomplete="off"
                                    placeholder=move || t(locale.get(), "runtime_config.python_placeholder")
                                    prop:value=move || form.get().map(|value| value.python_executable).unwrap_or_default()
                                    on:input=move |event| form.update(|value| {
                                        if let Some(value) = value {
                                            value.python_executable = event_target_value(&event);
                                        }
                                    }) />
                                {move || form.get().map(|value| value.context_kind == "local").unwrap_or(false).then(|| view! {
                                    <button type="button" class="runtime-config-browse" disabled=move || busy.get()
                                        on:click=move |_| browse("python")>{move || t(locale.get(), "runtime_config.browse")}</button>
                                })}
                            </div>
                        </label>
                        <label>
                            {move || t(locale.get(), "runtime_config.r")}
                            <div class="runtime-config-picker">
                                <input id="runtime-rscript-executable" autocomplete="off"
                                    placeholder=move || t(locale.get(), "runtime_config.r_placeholder")
                                    prop:value=move || form.get().map(|value| value.rscript_executable).unwrap_or_default()
                                    on:input=move |event| form.update(|value| {
                                        if let Some(value) = value {
                                            value.rscript_executable = event_target_value(&event);
                                        }
                                    }) />
                                {move || form.get().map(|value| value.context_kind == "local").unwrap_or(false).then(|| view! {
                                    <button type="button" class="runtime-config-browse" disabled=move || busy.get()
                                        on:click=move |_| browse("r")>{move || t(locale.get(), "runtime_config.browse")}</button>
                                })}
                            </div>
                        </label>
                        <p class="runtime-config-hint">{move || t(locale.get(), "runtime_config.hint")}</p>
                        {move || error.get().map(|message| view! {
                            <div class="settings-status fail">{message}</div>
                        })}
                        <div class="row">
                            <button type="button" disabled=move || busy.get()
                                on:click=move |_| form.set(None)>{move || t(locale.get(), "settings.cancel")}</button>
                            <button type="button" class="primary" disabled=move || busy.get()
                                on:click=save>{move || t(locale.get(), "settings.save")}</button>
                        </div>
                    </div>
                </div>
            })
    }
}

/// Root-owned open state (run id) for the run-review modal, shared through
/// the Leptos context so run cards anywhere can open it.
#[derive(Clone, Copy)]
pub(crate) struct RunReviewModal(pub(crate) RwSignal<Option<String>>);

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// Review a finished Run's server workspace: browse one directory level at a
/// time (server-side filter + paging, nothing persisted), download only the
/// explicit selection, delete the selection, or clean the whole workspace.
#[component]
pub(super) fn RunReviewOverlay(
    locale: RwSignal<Locale>,
    modal: RwSignal<Option<String>>,
    runs: RwSignal<Vec<RunSummary>>,
) -> impl IntoView {
    let path = create_rw_signal(String::new());
    let filter = create_rw_signal(String::new());
    let entries = create_rw_signal(Vec::<WorkspaceEntry>::new());
    let truncated = create_rw_signal(false);
    // Selected paths → kind ("file" | "dir").
    let selection = create_rw_signal(std::collections::HashMap::<String, String>::new());
    let busy = create_rw_signal(false);
    let error = create_rw_signal(None::<String>);
    let status = create_rw_signal(None::<String>);

    let fetch = move |append: bool| {
        let Some(run_id) = modal.get_untracked() else {
            return;
        };
        busy.set(true);
        error.set(None);
        let offset = if append {
            entries.with_untracked(|entries| entries.len())
        } else {
            0
        };
        let args = to_value(&serde_json::json!({
            "runId": run_id,
            "path": path.get_untracked(),
            "nameFilter": filter.get_untracked().trim(),
            "offset": offset,
            "limit": 200,
        }))
        .unwrap();
        spawn_local(async move {
            match invoke_checked("list_run_workspace_files", args).await {
                Ok(value) => {
                    if let Ok(listing) = serde_wasm_bindgen::from_value::<WorkspaceListing>(value) {
                        if append {
                            entries.update(|current| current.extend(listing.entries));
                        } else {
                            entries.set(listing.entries);
                        }
                        truncated.set(listing.truncated);
                    }
                }
                Err(value) => error.set(Some(localize_backend(
                    locale.get_untracked(),
                    &js_error_text(value),
                ))),
            }
            busy.set(false);
        });
    };

    // Fresh state and first page whenever the modal opens on a run.
    create_effect(move |previous: Option<Option<String>>| {
        let current = modal.get();
        if current.is_some() && previous.flatten() != current {
            path.set(String::new());
            filter.set(String::new());
            selection.set(Default::default());
            status.set(None);
            fetch(false);
        }
        current
    });

    let close = move || {
        modal.set(None);
        entries.set(Vec::new());
        selection.set(Default::default());
    };

    let download = move |_| {
        let Some(run_id) = modal.get_untracked() else {
            return;
        };
        let selected = selection.get_untracked();
        if selected.is_empty() {
            return;
        }
        let files: Vec<String> = selected
            .iter()
            .filter(|(_, kind)| kind.as_str() == "file")
            .map(|(path, _)| path.clone())
            .collect();
        let dirs: Vec<String> = selected
            .iter()
            .filter(|(_, kind)| kind.as_str() == "dir")
            .map(|(path, _)| path.clone())
            .collect();
        busy.set(true);
        error.set(None);
        let args = to_value(&serde_json::json!({
            "runId": run_id,
            "files": files,
            "dirs": dirs,
        }))
        .unwrap();
        spawn_local(async move {
            match invoke_checked("download_run_files", args).await {
                Ok(_) => {
                    selection.set(Default::default());
                    status.set(Some(t(locale.get_untracked(), "run_review.downloaded")));
                }
                Err(value) => error.set(Some(localize_backend(
                    locale.get_untracked(),
                    &js_error_text(value),
                ))),
            }
            busy.set(false);
        });
    };

    let delete = move |_| {
        let Some(run_id) = modal.get_untracked() else {
            return;
        };
        let selected: Vec<String> = selection.get_untracked().keys().cloned().collect();
        if selected.is_empty() {
            return;
        }
        busy.set(true);
        error.set(None);
        let args = to_value(&serde_json::json!({ "runId": run_id, "paths": selected })).unwrap();
        spawn_local(async move {
            match invoke_checked("delete_run_files", args).await {
                Ok(_) => {
                    selection.set(Default::default());
                    status.set(Some(t(locale.get_untracked(), "run_review.deleted")));
                    fetch(false);
                }
                Err(value) => {
                    error.set(Some(localize_backend(
                        locale.get_untracked(),
                        &js_error_text(value),
                    )));
                    busy.set(false);
                }
            }
        });
    };

    let cleanup_all = move |_| {
        let Some(run_id) = modal.get_untracked() else {
            return;
        };
        busy.set(true);
        error.set(None);
        // User-explicit whole-workspace cleanup accepts unretrieved data loss.
        let args = to_value(&serde_json::json!({ "runId": run_id, "force": true })).unwrap();
        spawn_local(async move {
            match invoke_checked("cleanup_run_workspace", args).await {
                Ok(_) => {
                    show_toast(&t(locale.get_untracked(), "runs.cleanup_done"));
                    refresh_runs(runs, locale);
                    modal.set(None);
                }
                Err(value) => error.set(Some(localize_backend(
                    locale.get_untracked(),
                    &js_error_text(value),
                ))),
            }
            busy.set(false);
        });
    };

    move || {
        modal.get().map(|run_id| {
            view! {
                <div class="overlay">
                    <div class="modal runs-details run-review-modal" data-testid="run-review-modal">
                        <div class="ps-head">
                            <div class="context-modal-title">
                                <h2>{move || t(locale.get(), "run_review.title")}</h2>
                                <code>{run_id.clone()}</code>
                            </div>
                            <button type="button" class="ps-close"
                                title=move || t(locale.get(), "settings.cancel")
                                on:click=move |_| close()>{compose_icon("close")}</button>
                        </div>
                        <p class="runtime-config-hint">{move || t(locale.get(), "run_review.hint")}</p>
                        <div class="run-review-toolbar">
                            {move || {
                                let current = path.get();
                                (!current.is_empty()).then(|| view! {
                                    <button type="button" class="run-review-up"
                                        disabled=move || busy.get()
                                        on:click=move |_| {
                                            path.update(|value| {
                                                *value = value.rsplit_once('/')
                                                    .map(|(parent, _)| parent.to_string())
                                                    .unwrap_or_default();
                                            });
                                            fetch(false);
                                        }>{move || t(locale.get(), "run_review.up")}</button>
                                })
                            }}
                            <code class="run-review-path">{move || {
                                let current = path.get();
                                if current.is_empty() { "/".to_string() } else { format!("/{current}") }
                            }}</code>
                            <input type="search" class="run-review-filter" autocomplete="off"
                                aria-label=move || t(locale.get(), "run_review.filter")
                                placeholder=move || t(locale.get(), "run_review.filter")
                                prop:value=move || filter.get()
                                on:input=move |event| {
                                    filter.set(event_target_value(&event));
                                    fetch(false);
                                } />
                        </div>
                        {move || error.get().map(|message| view! {
                            <div class="settings-status fail">{message}</div>
                        })}
                        {move || status.get().map(|message| view! {
                            <div class="settings-status ok" data-testid="run-review-status">{message}</div>
                        })}
                        <div class="run-review-list">
                            {move || {
                                let rows = entries.get();
                                if rows.is_empty() {
                                    view! { <div class="control-empty">{move || t(locale.get(), "run_review.empty")}</div> }.into_view()
                                } else {
                                    rows.into_iter().map(|entry| {
                                        let is_dir = entry.kind == "dir";
                                        let toggle_path = entry.path.clone();
                                        let toggle_kind = entry.kind.clone();
                                        let enter_path = entry.path.clone();
                                        let checked_path = entry.path.clone();
                                        let name = entry.path.rsplit('/').next().unwrap_or(&entry.path).to_string();
                                        let meta = if is_dir {
                                            format!(
                                                "{} · {} files",
                                                format_bytes(entry.size_bytes),
                                                entry.file_count.unwrap_or(0)
                                            )
                                        } else {
                                            format_bytes(entry.size_bytes)
                                        };
                                        view! {
                                            <div class="run-review-row" data-testid="run-review-row">
                                                <label class="run-review-select">
                                                    <input type="checkbox"
                                                        prop:checked=move || selection.with(|selected| selected.contains_key(&checked_path))
                                                        on:change=move |_| selection.update(|selected| {
                                                            if selected.remove(&toggle_path).is_none() {
                                                                selected.insert(toggle_path.clone(), toggle_kind.clone());
                                                            }
                                                        }) />
                                                </label>
                                                {if is_dir {
                                                    view! {
                                                        <button type="button" class="run-review-name run-review-dir"
                                                            on:click=move |_| {
                                                                path.set(enter_path.clone());
                                                                fetch(false);
                                                            }>{compose_icon("folder")}<span>{name}</span></button>
                                                    }.into_view()
                                                } else {
                                                    view! { <span class="run-review-name">{name}</span> }.into_view()
                                                }}
                                                <span class="run-review-meta">{meta}</span>
                                            </div>
                                        }.into_view()
                                    }).collect_view()
                                }
                            }}
                            {move || truncated.get().then(|| view! {
                                <button type="button" class="run-review-more" disabled=move || busy.get()
                                    on:click=move |_| fetch(true)>{move || t(locale.get(), "run_review.more")}</button>
                            })}
                        </div>
                        <p class="runtime-config-hint">{move || t(locale.get(), "run_review.delete_warning")}</p>
                        <div class="row">
                            <button type="button" class="run-review-cleanup"
                                disabled=move || busy.get()
                                on:click=cleanup_all>{move || t(locale.get(), "run_review.cleanup_all")}</button>
                            <button type="button" class="run-review-delete"
                                disabled=move || busy.get() || selection.with(|selected| selected.is_empty())
                                on:click=delete>{move || t(locale.get(), "run_review.delete_selected")}</button>
                            <button type="button" class="primary run-review-download"
                                disabled=move || busy.get() || selection.with(|selected| selected.is_empty())
                                on:click=download>{move || t(locale.get(), "run_review.download_selected")}</button>
                        </div>
                    </div>
                </div>
            }
        })
    }
}

/// Storage locations for one project × server: where uploads land, where run
/// workdirs live, and where retrieved outputs are placed in the project.
/// Auto-opens once when a server is first enabled without saved preferences.
#[component]
pub(super) fn StoragePrefsOverlay(
    locale: RwSignal<Locale>,
    form: RwSignal<Option<StoragePrefsForm>>,
) -> impl IntoView {
    let busy = create_rw_signal(false);
    let error = create_rw_signal(None::<String>);
    // Render from the open state only so typing does not rebuild the DOM.
    let open = create_memo(move |_| form.with(|value| value.is_some()));
    let save = move |_| {
        let Some(current) = form.get_untracked() else {
            return;
        };
        busy.set(true);
        error.set(None);
        let args = to_value(&serde_json::json!({
            "contextId": current.context_id,
            "remoteDataRoot": current.remote_data_root,
            "remoteWorkdirRoot": current.remote_workdir_root,
            "localResultsDir": current.local_results_dir,
        }))
        .unwrap();
        spawn_local(async move {
            match invoke_checked("set_context_storage_prefs", args).await {
                Ok(_) => {
                    form.set(None);
                    show_toast(&t(locale.get_untracked(), "storage_prefs.saved"));
                }
                Err(value) => error.set(Some(localize_backend(
                    locale.get_untracked(),
                    &js_error_text(value),
                ))),
            }
            busy.set(false);
        });
    };
    let field = move |name: &'static str, event: &leptos::ev::Event| {
        let value = event_target_value(event);
        form.update(|current| {
            if let Some(current) = current {
                match name {
                    "data" => current.remote_data_root = value,
                    "workdir" => current.remote_workdir_root = value,
                    _ => current.local_results_dir = value,
                }
            }
        });
    };

    move || {
        open.get().then(|| view! {
            <div class="overlay">
                <div class="modal runtime-config-modal storage-prefs-modal" data-testid="storage-prefs-modal">
                    <div class="ps-head">
                        <h2>{move || t(locale.get(), "storage_prefs.title")}</h2>
                        <button type="button" class="ps-close"
                            title=move || t(locale.get(), "settings.cancel")
                            disabled=move || busy.get()
                            on:click=move |_| form.set(None)>{compose_icon("close")}</button>
                    </div>
                    <p class="runtime-config-hint">{
                        move || {
                            let context = form.with(|value| value.as_ref()
                                .map(|value| value.context_label.clone())
                                .unwrap_or_default());
                            tf(locale.get(), "storage_prefs.scope", &[("context", &context)])
                        }
                    }</p>
                    <label>
                        {move || t(locale.get(), "storage_prefs.data_root")}
                        <input id="storage-prefs-data-root" autocomplete="off"
                            prop:value=move || form.get().map(|value| value.remote_data_root).unwrap_or_default()
                            on:input=move |event| field("data", &event) />
                    </label>
                    <label>
                        {move || t(locale.get(), "storage_prefs.workdir_root")}
                        <input id="storage-prefs-workdir-root" autocomplete="off"
                            prop:value=move || form.get().map(|value| value.remote_workdir_root).unwrap_or_default()
                            on:input=move |event| field("workdir", &event) />
                    </label>
                    <label>
                        {move || t(locale.get(), "storage_prefs.results_dir")}
                        <input id="storage-prefs-results-dir" autocomplete="off"
                            prop:value=move || form.get().map(|value| value.local_results_dir).unwrap_or_default()
                            on:input=move |event| field("results", &event) />
                    </label>
                    <p class="runtime-config-hint">{move || t(locale.get(), "storage_prefs.hint")}</p>
                    {move || error.get().map(|message| view! {
                        <div class="settings-status fail">{message}</div>
                    })}
                    <div class="row">
                        <button type="button" disabled=move || busy.get()
                            on:click=move |_| form.set(None)>{
                                move || if form.with(|value| value.as_ref().map(|value| value.first_use).unwrap_or(false)) {
                                    t(locale.get(), "storage_prefs.later")
                                } else {
                                    t(locale.get(), "settings.cancel")
                                }
                            }</button>
                        <button type="button" class="primary" disabled=move || busy.get()
                            on:click=save>{move || t(locale.get(), "settings.save")}</button>
                    </div>
                </div>
            </div>
        })
    }
}

#[component]
pub(super) fn CapabilitiesOverlay(
    locale: RwSignal<Locale>,
    show_capabilities: RwSignal<bool>,
    bootstrap: RwSignal<Option<BootstrapStatus>>,
    caps: RwSignal<Option<Capabilities>>,
    busy: RwSignal<bool>,
    open_settings_section: Callback<String>,
    start_env_setup: Callback<web_sys::MouseEvent>,
) -> impl IntoView {
    move || {
        show_capabilities.get().then(|| view! {
    <div class="overlay">
        <div class="modal modal-wide" role="dialog" aria-modal="true"
            aria-labelledby="capabilities-title">
            <div class="ps-head">
                <h2 id="capabilities-title">{move || t(locale.get(), "caps.title")}</h2>
                <button type="button" class="ps-close"
                    title=move || t(locale.get(), "caps.close")
                    aria-label=move || t(locale.get(), "caps.close")
                    on:click=move |_| show_capabilities.set(false)>
                    {compose_icon("close")}
                </button>
            </div>
            {move || bootstrap.get().map(|b| {
                let loc = locale.get();
                view! {
                <div class="cap-section">
                    <h3>{tf(loc, "caps.runtime", &[("version", &b.app_version)])}</h3>
                    <p class="hint">{tf(loc, "caps.workspace", &[("path", &b.workspace)])}</p>
                    <p class="hint">{{
                        let ready = t(loc, "caps.ready");
                        let missing = t(loc, "caps.missing");
                        tf(loc, "caps.runtime_status", &[
                        ("py", if b.python_ok { &ready } else { &missing }),
                        ("uv", if b.uv_ok { &ready } else { &missing }),
                        ("node", if b.node_ok { &ready } else { &missing }),
                        ("sci", if b.sci_ok { &ready } else { &missing }),
                        ("pixi", if b.pixi_ok { &ready } else { &missing }),
                        ("skills", &b.skills_loaded.to_string()),
                        ("mcp", &b.mcp_catalog.to_string()),
                    ])}}</p>
                    {(!b.errors.is_empty()).then(|| view! {
                        <div class="settings-status fail">
                            {b.errors.join("\n")}
                        </div>
                    })}
                </div>
            }})}
            {move || caps.get().map(|c| view! {
                // ponytail: counts only — detail lists (bio-tool tags, skill list,
                // permissions hint) live in Settings, not this read-only summary.
                <div class="cap-grid">
                    <button type="button" class="cap-stat"
                        on:click=move |_| {
                            show_capabilities.set(false);
                            open_settings_section.call("skills".into());
                        }>
                        <span class="cap-num">{c.skill_counts.bundled}</span>
                        <span class="cap-label">{move || t(locale.get(), "caps.bundled_skills")}</span>
                    </button>
                    <button type="button" class="cap-stat"
                        on:click=move |_| {
                            show_capabilities.set(false);
                            open_settings_section.call("skills".into());
                        }>
                        <span class="cap-num">{c.skill_counts.project}</span>
                        <span class="cap-label">{move || t(locale.get(), "caps.project_skills")}</span>
                    </button>
                    <button type="button" class="cap-stat"
                        on:click=move |_| {
                            show_capabilities.set(false);
                            open_settings_section.call("connections".into());
                        }>
                        <span class="cap-num">{c.mcp_counts.bundled}</span>
                        <span class="cap-label">{move || t(locale.get(), "caps.bundled_mcp_servers")}</span>
                    </button>
                    <button type="button" class="cap-stat"
                        on:click=move |_| {
                            show_capabilities.set(false);
                            open_settings_section.call("connections".into());
                        }>
                        <span class="cap-num">{c.mcp_counts.project}</span>
                        <span class="cap-label">{move || t(locale.get(), "caps.project_mcp_servers")}</span>
                    </button>
                    <button type="button" class="cap-stat"
                        on:click=move |_| {
                            show_capabilities.set(false);
                            open_settings_section.call("memory".into());
                        }>
                        <span class="cap-num">{c.memory_files.len()}</span>
                        <span class="cap-label">{move || t(locale.get(), "caps.memory_files")}</span>
                    </button>
                </div>
            })}
            <div class="row">
                <button on:click=move |_| show_capabilities.set(false)>
                    {move || t(locale.get(), "caps.close")}
                </button>
                {move || bootstrap.get().filter(|b| !b.python_initializing && (!b.python_ok || !b.uv_ok || !b.node_ok || !b.sci_ok || !b.pixi_ok)).map(|_| view! {
                    <button class="primary" disabled=move || busy.get() on:click=move |ev| start_env_setup.call(ev)>
                        {move || t(locale.get(), "caps.setup_env")}
                    </button>
                })}
            </div>
        </div>
    </div>
})
    }
}

// ponytail: onboarding only offers DeepSeek; other providers live in Settings › Models.
const DEEPSEEK_KEY_URL: &str = "https://platform.deepseek.com/api_keys";

#[component]
pub(super) fn OnboardingOverlay(
    locale: RwSignal<Locale>,
    show_onboarding: RwSignal<bool>,
    onboard_step: RwSignal<usize>,
    onboard_key: RwSignal<String>,
    save_onboard_key: Callback<()>,
    dismiss_onboard: Callback<web_sys::MouseEvent>,
) -> impl IntoView {
    move || {
        show_onboarding.get().then(|| {
    let step = onboard_step.get();
    let loc = locale.get();
    view! {
        <div class="overlay onboard-overlay">
            <div class="modal onboard">
                {match step {
                    0 => view! {
                        <h2>{t(loc, "onboard.apikey.title")}</h2>
                        <ol class="onboard-steps">
                            <li>
                                <p class="hint">{t(loc, "onboard.getkey.body")}</p>
                                <button type="button" class="linklike onboard-getkey"
                                    on:click=move |_| open_external_url(DEEPSEEK_KEY_URL.into())>
                                    {t(loc, "onboard.apikey.get_key")}
                                </button>
                            </li>
                            <li>
                                <p class="hint">{t(loc, "onboard.apikey.body")}</p>
                                <label>{t(loc, "settings.api_key")}
                                    <input type="password" autocomplete="new-password"
                                        prop:value=move || onboard_key.get()
                                        on:input=move |ev| onboard_key.set(event_target_value(&ev)) />
                                </label>
                            </li>
                        </ol>
                    }.into_view(),
                    1 => view! {
                        <h2>{t(loc, "onboard.welcome.title")}</h2>
                        <p class="hint">{t(loc, "onboard.welcome.body")}</p>
                    }.into_view(),
                    _ => view! {
                        <h2>{t(loc, "onboard.features.title")}</h2>
                        <p class="hint">{t(loc, "onboard.features.body")}</p>
                    }.into_view(),
                }}
                <div class="onboard-dots">
                    {(0..3).map(|i| view! {
                        <span class="onboard-dot" class:active=move || onboard_step.get() == i></span>
                    }).collect_view()}
                </div>
                <div class="row">
                    {if step > 0 {
                        view! { <button on:click=move |_| onboard_step.update(|s| *s = s.saturating_sub(1))>{move || t(locale.get(), "onboard.back")}</button> }.into_view()
                    } else { view! { <span></span> }.into_view() }}
                    {if step < 2 {
                        view! { <button class="primary" on:click=move |_| {
                            if step == 0 { save_onboard_key.call(()); }
                            onboard_step.update(|s| *s += 1);
                        }>{move || t(locale.get(), if step == 0 && onboard_key.get().trim().is_empty() {
                            "onboard.apikey.later"
                        } else { "onboard.next" })}</button> }.into_view()
                    } else {
                        view! {
                            <button class="primary" on:click=move |ev| dismiss_onboard.call(ev)>{move || t(locale.get(), "onboard.start")}</button>
                        }.into_view()
                    }}
                </div>
            </div>
        </div>
    }.into_view()
})
    }
}
