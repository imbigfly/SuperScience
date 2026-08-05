use crate::app_support::{
    compose_icon, inspect_runtime_objects, language_display, runtime_binding_state_key,
    runtime_object_quote, runtime_status_label, RuntimeConsoles,
};
use crate::bindings::{mount_terminal, set_terminal_active, unmount_terminal};
use crate::dto::{RuntimeInfo, RuntimeObjectState};
use crate::i18n::{t, tf, use_locale, Locale};
use crate::text::format_bytes;
use futures_channel::oneshot;
use leptos::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

#[component]
pub(crate) fn CenterRuntimeConsole(
    path: String,
    consoles: RwSignal<RuntimeConsoles>,
) -> impl IntoView {
    let locale = use_locale();
    let log_path = path.clone();
    let clear_path = path;
    let log = create_memo(move |_| consoles.get().get(&log_path).cloned().unwrap_or_default());
    let output_ref = create_node_ref::<html::Pre>();

    // Follow appended output with ordinary positive scrollTop. The old
    // column-reverse trick made WebKit's scrollbar direction and selection
    // behavior backwards, especially once a whole script filled the console.
    create_effect(move |_| {
        let _ = log.get();
        if let Some(output) = output_ref.get() {
            request_animation_frame(move || output.set_scroll_top(output.scroll_height()));
        }
    });

    view! {
        <div class="center-file-console">
            <div class="center-file-console-head">
                <span>{move || t(locale.get(), "runtime.console")}</span>
                <div class="spacer"></div>
                <button type="button" class="center-file-btn"
                    title=move || t(locale.get(), "runtime.console_clear")
                    aria-label=move || t(locale.get(), "runtime.console_clear")
                    on:click=move |_| consoles.update(|logs| {
                        logs.remove(&clear_path);
                    })>{compose_icon("close")}</button>
            </div>
            <pre node_ref=output_ref class:empty=move || log.get().is_empty()>{move || {
                let text = log.get();
                if text.is_empty() {
                    t(locale.get(), "runtime.console_empty").into()
                } else {
                    text
                }
            }}</pre>
        </div>
    }
}

#[component]
pub(crate) fn CenterRuntimeEnvironment(
    project_id: String,
    context_id: String,
    context_label: String,
    language: String,
    locale: RwSignal<Locale>,
    states: RwSignal<HashMap<String, RuntimeObjectState>>,
    runtimes: RwSignal<Vec<RuntimeInfo>>,
    selection_popup: RwSignal<Option<(String, Option<String>, i32, i32)>>,
) -> impl IntoView {
    let state_key = runtime_binding_state_key(&project_id, &context_id, &language);
    let status_project = project_id.clone();
    let status_context = context_id.clone();
    let status_language = language.clone();
    let status = create_memo(move |_| {
        runtimes
            .get()
            .into_iter()
            .find(|runtime| {
                runtime.key.project_id == status_project
                    && runtime.key.context_id == status_context
                    && runtime.key.language == status_language
            })
            .map(|runtime| runtime.status)
            .unwrap_or_else(|| "missing".into())
    });
    let language_label = language_display(&language).to_string();
    let aria_language_label = language_label.clone();
    let title_language_label = language_label.clone();
    let loading_key = state_key.clone();
    let content_key = state_key.clone();
    let refresh_key = state_key;
    let refresh_project = project_id;
    let refresh_context = context_id;
    let refresh_language = language;

    view! {
        <aside class="center-runtime-environment" aria-label=move || {
            tf(locale.get(), "runtime.environment_title", &[("language", &aria_language_label)])
        }>
            <div class="center-runtime-environment-head">
                <div>
                    <h3>{move || tf(locale.get(), "runtime.environment_title", &[("language", &title_language_label)])}</h3>
                    <span>{context_label}</span>
                </div>
                <span class=move || format!("runtime-status {}", status.get())>
                    {move || runtime_status_label(locale.get(), &status.get())}
                </span>
                <button type="button" class="runtime-environment-refresh"
                    title=move || t(locale.get(), "runtime.inspect_objects")
                    aria-label=move || t(locale.get(), "runtime.inspect_objects")
                    disabled=move || status.get() != "ready" || states.with(|states| {
                        states.get(&loading_key).is_some_and(|state| state.loading)
                    })
                    on:click=move |_| inspect_runtime_objects(
                        refresh_key.clone(),
                        refresh_project.clone(),
                        refresh_context.clone(),
                        refresh_language.clone(),
                        locale,
                        states,
                        runtimes,
                    )>{compose_icon("sync")}</button>
            </div>
            <div class="center-runtime-environment-table-head" aria-hidden="true">
                <span>{move || t(locale.get(), "runtime.object_name")}</span>
                <span>{move || t(locale.get(), "runtime.object_type")}</span>
                <span>{move || t(locale.get(), "runtime.object_value")}</span>
                <span>{move || t(locale.get(), "runtime.object_size")}</span>
            </div>
            <div class="center-runtime-environment-body">
                {move || {
                    let state = states.with(|states| {
                        states.get(&content_key).cloned().unwrap_or_default()
                    });
                    if state.loading && state.snapshot.is_none() {
                        return view! {
                            <div class="runtime-environment-empty">{t(locale.get(), "runtime.objects_loading")}</div>
                        }.into_view();
                    }
                    if let Some(error) = state.error {
                        return view! { <div class="context-error">{error}</div> }.into_view();
                    }
                    let Some(snapshot) = state.snapshot else {
                        let key = if status.get() == "ready" {
                            "runtime.objects_hint"
                        } else {
                            "runtime.environment_unavailable"
                        };
                        return view! {
                            <div class="runtime-environment-empty">{t(locale.get(), key)}</div>
                        }.into_view();
                    };
                    if snapshot.objects.is_empty() {
                        return view! {
                            <div class="runtime-environment-empty">{t(locale.get(), "runtime.objects_empty")}</div>
                        }.into_view();
                    }
                    let shown = snapshot.objects.len();
                    let total = snapshot.total_count;
                    view! {
                        <div class="center-runtime-environment-rows">
                            {snapshot.objects.into_iter().map(|object| {
                                let size = object.size_bytes.map(format_bytes).unwrap_or_else(|| "—".into());
                                let summary = if object.summary.is_empty() { "—".into() } else { object.summary };
                                let quote = runtime_object_quote(
                                    &language_label, &object.name, &object.type_name, &summary, &size,
                                );
                                view! {
                                    <div class="center-runtime-environment-row" role="button" tabindex="0"
                                        title=move || t(locale.get(), "runtime.quote_object")
                                        on:click=move |event: web_sys::MouseEvent| selection_popup.set(Some((
                                            quote.clone(), None, event.client_x(), event.client_y(),
                                        )))>
                                        <span class="runtime-object-name" title=object.name.clone()>{object.name}</span>
                                        <span class="runtime-object-type" title=object.type_name.clone()>{object.type_name}</span>
                                        <span class="runtime-object-value" title=summary.clone()>{summary}</span>
                                        <span class="runtime-object-size">{size}</span>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                        {(shown < total).then(|| view! {
                            <div class="runtime-objects-limit">{
                                tf(locale.get(), "runtime.objects_showing", &[
                                    ("shown", &shown.to_string()),
                                    ("total", &total.to_string()),
                                ])
                            }</div>
                        })}
                    }.into_view()
                }}
            </div>
        </aside>
    }
}

#[derive(Default)]
pub(crate) struct ProjectOpenGate {
    held: bool,
    waiters: VecDeque<oneshot::Sender<()>>,
}

pub(crate) struct ProjectOpenPermit(Rc<RefCell<ProjectOpenGate>>);

impl Drop for ProjectOpenPermit {
    fn drop(&mut self) {
        let next = self.0.borrow_mut().waiters.pop_front();
        if let Some(next) = next {
            let _ = next.send(());
        } else {
            self.0.borrow_mut().held = false;
        }
    }
}

pub(crate) async fn acquire_project_open_gate(
    gate: Rc<RefCell<ProjectOpenGate>>,
) -> ProjectOpenPermit {
    let receiver = {
        let mut state = gate.borrow_mut();
        if state.held {
            let (sender, receiver) = oneshot::channel();
            state.waiters.push_back(sender);
            Some(receiver)
        } else {
            state.held = true;
            None
        }
    };
    if let Some(receiver) = receiver {
        let _ = receiver.await;
    }
    ProjectOpenPermit(gate)
}

pub(crate) fn project_transition_is_current(
    epoch: &Rc<Cell<u64>>,
    target: &Rc<RefCell<Option<String>>>,
    request_epoch: u64,
    project_id: &str,
) -> bool {
    epoch.get() == request_epoch && target.borrow().as_deref() == Some(project_id)
}

pub(crate) fn terminal_element_id(session_id: &str) -> String {
    format!("terminal-session-{session_id}")
}

pub(crate) fn terminal_tab_id(session_id: &str) -> String {
    format!("terminal-tab-{session_id}")
}

#[component]
pub(crate) fn TerminalHost(
    session_id: String,
    active_terminal_id: RwSignal<Option<String>>,
) -> impl IntoView {
    let element_id = terminal_element_id(&session_id);
    let labelled_by = terminal_tab_id(&session_id);
    let host_ref = create_node_ref::<html::Div>();
    let mount_element_id = element_id.clone();
    let mount_session_id = session_id.clone();
    let active_session_id = session_id.clone();
    let class_session_id = session_id.clone();

    create_effect(move |_| {
        if host_ref.get().is_none() {
            return;
        }
        mount_terminal(&mount_element_id, &mount_session_id);
        set_terminal_active(
            &mount_element_id,
            active_terminal_id.get().as_deref() == Some(active_session_id.as_str()),
        );
    });

    let cleanup_element_id = element_id.clone();
    on_cleanup(move || unmount_terminal(&cleanup_element_id));

    view! {
        <div
            id=element_id
            node_ref=host_ref
            class="terminal-dock-frame"
            class:active=move || active_terminal_id.get().as_deref() == Some(class_session_id.as_str())
            data-terminal-session=session_id
            role="tabpanel"
            aria-labelledby=labelled_by
        ></div>
    }
}
