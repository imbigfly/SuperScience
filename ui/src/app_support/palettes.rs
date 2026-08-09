use super::*;

pub(crate) const LITERATURE_RESEARCH_ACTION_ID: &str = "literature_research";
pub(crate) const LITERATURE_REVIEW_SKILL: &str = "literature-review";

/// Apply a transcript mutation to the right session: the live `items` view when
/// `fid` is the active session, otherwise the background cache keyed by `fid`.
/// This is what lets a second conversation stream while the user views another.
pub(crate) fn route_items(
    active: RwSignal<Option<String>>,
    items: RwSignal<Vec<ChatItem>>,
    transcripts: RwSignal<HashMap<String, Vec<ChatItem>>>,
    fid: &str,
    f: impl FnOnce(&mut Vec<ChatItem>),
) {
    if active.get().as_deref() == Some(fid) {
        items.update(f);
    } else {
        transcripts.update(|m| f(m.entry(fid.to_string()).or_insert_with(Vec::new)));
    }
}

pub(crate) fn quick_action_label(locale: Locale, action: &QuickAction) -> String {
    if action.id == LITERATURE_RESEARCH_ACTION_ID && action.name == "Research literature" {
        t(locale, "selection.research_literature").into()
    } else {
        action.name.clone()
    }
}

/// The built-in literature action stays in the current conversation so its
/// progress and result share the conversation's durable transcript. Custom
/// Quick Actions continue to instantiate their bound Workflow.
pub(crate) fn quick_action_uses_current_conversation(action: &QuickAction) -> bool {
    action.builtin && action.id == LITERATURE_RESEARCH_ACTION_ID
}

pub(crate) fn append_composer_prompt(current: &str, prompt: &str) -> String {
    let prompt = prompt.trim();
    let current_text = current.trim();
    if current_text.is_empty() {
        prompt.to_string()
    } else if prompt.is_empty() || current_text.ends_with(prompt) {
        current.to_string()
    } else {
        format!("{}\n\n{prompt}", current.trim_end())
    }
}

#[cfg(test)]
mod quick_action_routing_tests {
    use super::*;

    fn action(id: &str, builtin: bool) -> QuickAction {
        QuickAction {
            id: id.into(),
            name: "Action".into(),
            description: String::new(),
            icon: "search".into(),
            context: "selection".into(),
            workflow_template_id: "workflow".into(),
            enabled: true,
            sort_order: 0,
            builtin,
        }
    }

    #[test]
    fn only_builtin_literature_research_stays_in_current_conversation() {
        assert!(quick_action_uses_current_conversation(&action(
            LITERATURE_RESEARCH_ACTION_ID,
            true,
        )));
        assert!(!quick_action_uses_current_conversation(&action(
            LITERATURE_RESEARCH_ACTION_ID,
            false,
        )));
        assert!(!quick_action_uses_current_conversation(&action("custom", true)));
    }

    #[test]
    fn composer_prompt_preserves_an_existing_draft() {
        assert_eq!(append_composer_prompt("", "Research this"), "Research this");
        assert_eq!(
            append_composer_prompt("Keep this context", "Research this"),
            "Keep this context\n\nResearch this"
        );
        assert_eq!(
            append_composer_prompt("Keep this context\n\nResearch this", "Research this"),
            "Keep this context\n\nResearch this"
        );
    }
}

pub(crate) fn selection_popup_x(x: i32) -> i32 {
    let Some(viewport) = web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|value| value.as_f64())
        .map(|value| value.round() as i32)
    else {
        return x;
    };
    let usable = (viewport - 24).max(0);
    let half_width = usable.min(720) / 2;
    x.clamp(half_width, viewport - half_width)
}

pub(crate) fn selection_popup_y(y: i32) -> i32 {
    let Some(viewport) = web_sys::window()
        .and_then(|window| window.inner_height().ok())
        .and_then(|value| value.as_f64())
        .map(|value| value.round() as i32)
    else {
        return y;
    };
    let max_y = (viewport - 12).max(0);
    y.clamp(120.min(max_y), max_y)
}

/// A dedicated project window (#52) carries `?project=<id>` in its URL. Returns
/// that id so the window opens straight into the project and skips the landing.
/// Project ids are UUIDs or "default" — no percent-decoding needed.
pub(crate) fn url_project_param() -> Option<String> {
    url_query_param("project")
}

/// Optional `&session=<id>` companion to `?project=` (#423): the window opens
/// straight into that session. Session ids are UUIDs — no percent-decoding.
pub(crate) fn url_session_param() -> Option<String> {
    url_query_param("session")
}

fn url_query_param(key: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    let q = search.strip_prefix('?').unwrap_or(&search);
    let prefix = format!("{key}=");
    q.split('&')
        .find_map(|p| p.strip_prefix(prefix.as_str()))
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

#[component]
pub(crate) fn CommandPalette(
    open: RwSignal<bool>,
    current_project_id: Signal<Option<String>>,
    on_open_project: Callback<(String, bool)>,
    on_open_session: Callback<(String, String, bool)>,
    on_open_artifact: Callback<(String, String, String)>,
    on_command: Callback<&'static str>,
    on_new_session: Callback<()>,
    on_open_scratch: Callback<()>,
    on_project_settings: Callback<()>,
    on_manage_skills: Callback<()>,
    on_attach: Callback<ComposerReferenceChip>,
) -> impl IntoView {
    let locale = use_locale();
    let query = create_rw_signal(String::new());
    let active = create_rw_signal(0usize);
    let projects = create_rw_signal(Vec::<ProjectSummary>::new());
    let artifacts = create_rw_signal(Vec::<ArtifactInfo>::new());
    let sessions = create_rw_signal(Vec::<SessionSearchInfo>::new());
    create_effect(move |_| {
        if !open.get() {
            return;
        }
        let q = query.get();
        spawn_local(async move {
            let p = invoke("list_projects", JsValue::UNDEFINED).await;
            if let Ok(rows) = serde_wasm_bindgen::from_value::<Vec<ProjectSummary>>(p) {
                projects.set(rows);
            }
            let a = invoke(
                "search_artifacts",
                to_value(&serde_json::json!({ "query": q, "limit": 12, "allProjects": true }))
                    .unwrap(),
            )
            .await;
            if query.get_untracked() == q {
                if let Ok(rows) = serde_wasm_bindgen::from_value::<Vec<ArtifactInfo>>(a) {
                    artifacts.set(rows);
                }
            }
            let s = invoke(
                "search_sessions",
                to_value(&serde_json::json!({ "query": q, "limit": 12 })).unwrap(),
            )
            .await;
            if query.get_untracked() == q {
                if let Ok(rows) = serde_wasm_bindgen::from_value::<Vec<SessionSearchInfo>>(s) {
                    sessions.set(rows);
                }
            }
        });
    });
    create_effect(move |_| {
        open.get();
        query.get();
        active.set(0);
    });
    create_effect(move |_| {
        if open.get() {
            focus_element_soon("command-palette-input");
        }
    });
    let items = create_memo(move |_| {
        let q = query.get().trim().to_lowercase();
        let current = current_project_id.get();
        let mut out = Vec::new();
        let mut ps: Vec<_> = projects
            .get()
            .into_iter()
            .filter(|p| contains_search(&q, &[&p.name, &p.description]))
            .collect();
        ps.sort_by_key(|p| (current.as_deref() != Some(p.id.as_str()), p.name.clone()));
        out.extend(ps.into_iter().map(CommandPaletteItem::Project));
        let mut ars = artifacts.get();
        ars.sort_by_key(|a| {
            (
                current.as_deref() != a.project_id.as_deref(),
                std::cmp::Reverse(a.ts),
            )
        });
        out.extend(ars.into_iter().map(CommandPaletteItem::Artifact));
        let mut ss = sessions.get();
        if q.is_empty() {
            // Empty query is the "switch away" gesture (#423): most recent
            // activity first across all projects, no current-project bias.
            ss.sort_by_key(|s| std::cmp::Reverse(s.activity_at));
        } else {
            ss.sort_by_key(|s| {
                (
                    current.as_deref() != Some(s.project_id.as_str()),
                    std::cmp::Reverse(s.activity_at),
                )
            });
        }
        out.extend(ss.into_iter().map(CommandPaletteItem::Session));
        out.push(CommandPaletteItem::Command("scratch"));
        out.push(CommandPaletteItem::Command("new"));
        out.push(CommandPaletteItem::Command("check-updates"));
        out.push(CommandPaletteItem::Command("star-us"));
        if current.is_some() {
            out.push(CommandPaletteItem::Command("settings"));
            out.push(CommandPaletteItem::Command("skills"));
        }
        out
    });
    let open_item = Callback::new(move |(idx, new_window): (usize, bool)| {
        let Some(item) = items.get().get(idx).cloned() else {
            return;
        };
        open.set(false);
        match item {
            CommandPaletteItem::Project(p) => on_open_project.call((p.id, new_window)),
            CommandPaletteItem::Artifact(a) => {
                let kind = file_kind(&a.name)
                    .or_else(|| file_kind(&a.path))
                    .unwrap_or("text")
                    .to_string();
                on_open_artifact.call((format!("artifact:{}", a.id), a.name, kind));
            }
            CommandPaletteItem::Session(s) => {
                on_open_session.call((s.project_id, s.id, new_window))
            }
            CommandPaletteItem::Command("scratch") => on_open_scratch.call(()),
            CommandPaletteItem::Command("new") => on_new_session.call(()),
            CommandPaletteItem::Command("check-updates") => on_command.call("check-updates"),
            CommandPaletteItem::Command("star-us") => on_command.call("star-us"),
            CommandPaletteItem::Command("settings") => on_project_settings.call(()),
            CommandPaletteItem::Command("skills") => on_manage_skills.call(()),
            CommandPaletteItem::Command(_) => {}
        }
    });
    let attach_item = Callback::new(move |idx: usize| {
        let list = items.get();
        let item = list
            .get(idx)
            .cloned()
            .filter(|item| {
                matches!(
                    item,
                    CommandPaletteItem::Artifact(_) | CommandPaletteItem::Session(_)
                )
            })
            .or_else(|| {
                list.into_iter().find(|item| {
                    matches!(
                        item,
                        CommandPaletteItem::Artifact(_) | CommandPaletteItem::Session(_)
                    )
                })
            });
        let Some(item) = item else {
            return;
        };
        match item {
            CommandPaletteItem::Artifact(a) => on_attach.call(ComposerReferenceChip::Artifact {
                id: a.id,
                name: a.name,
            }),
            CommandPaletteItem::Session(s) => on_attach.call(ComposerReferenceChip::Session {
                id: s.id,
                title: s.title,
                project_name: s.project_name,
            }),
            _ => return,
        }
        open.set(false);
        focus_composer();
    });
    view! {
        {move || open.get().then(|| view! {
            <div class="project-search-overlay" on:click=move |_| open.set(false)>
                <div class="project-search-dialog" role="dialog" aria-label="Search"
                    on:click=|ev| ev.stop_propagation()>
                    <div class="project-search-input">
                        <span class="gi search"></span>
                        <input id="command-palette-input" type="text" inputmode="search" autofocus=true
                            autocomplete="off" autocorrect="off" autocapitalize="none" spellcheck="false"
                            placeholder=move || t(locale.get(), "command.search_ph")
                            prop:value=move || query.get()
                            on:input=move |ev| query.set(event_target_value(&ev))
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if ime_composing(&ev) { return; }
                                let n = items.get().len();
                                match ev.key().as_str() {
                                    "Escape" => { ev.prevent_default(); open.set(false); }
                                    "ArrowDown" => { ev.prevent_default(); if n > 0 { let next = (active.get() + 1) % n; active.set(next); scroll_picker_item(".project-search-dialog:not(.action-palette) .project-search-row", next); } }
                                    "ArrowUp" => { ev.prevent_default(); if n > 0 { let next = (active.get() + n - 1) % n; active.set(next); scroll_picker_item(".project-search-dialog:not(.action-palette) .project-search-row", next); } }
                                    "Enter" if ev.shift_key() => { ev.prevent_default(); attach_item.call(active.get()); }
                                    "Enter" if ev.ctrl_key() || ev.meta_key() => { ev.prevent_default(); open_item.call((active.get(), true)); }
                                    "Enter" => { ev.prevent_default(); open_item.call((active.get(), false)); }
                                    _ => {}
                                }
                            } />
                    </div>
                    <div class="project-search-results">
                        {move || items.get().into_iter().enumerate().map(|(i, item)| {
                            let opens_project_window = matches!(&item, CommandPaletteItem::Project(_) | CommandPaletteItem::Session(_));
                            let (icon, title, sub) = match item {
                                CommandPaletteItem::Project(p) => ("folder", p.name, p.description),
                                CommandPaletteItem::Artifact(a) => ("doc", a.name, a.project_name.unwrap_or_default()),
                                CommandPaletteItem::Session(s) => ("bubble", s.title, s.project_name),
                                CommandPaletteItem::Command("scratch") => ("bubble", t(locale.get(), "command.scratch").to_string(), t(locale.get(), "command.category")),
                                CommandPaletteItem::Command("new") => ("plus", t(locale.get(), "projects.new").to_string(), t(locale.get(), "command.category")),
                                CommandPaletteItem::Command("check-updates") => ("gear", t(locale.get(), "command.check_updates").to_string(), t(locale.get(), "command.category")),
                                CommandPaletteItem::Command("star-us") => ("star", t(locale.get(), "command.star_us").to_string(), t(locale.get(), "command.category")),
                                CommandPaletteItem::Command("settings") => ("gear", t(locale.get(), "proj_settings.title").to_string(), t(locale.get(), "command.category")),
                                CommandPaletteItem::Command("skills") => ("grid", t(locale.get(), "settings.nav.skills").to_string(), t(locale.get(), "command.category")),
                                CommandPaletteItem::Command(_) => ("doc", String::new(), String::new()),
                            };
                            view! {
                                <button type="button" class="project-search-row" class:active=move || active.get() == i
                                    on:mousemove=move |_| active.set(i)
                                    on:click=move |_| open_item.call((i, false))>
                                    <span class=format!("gi {icon}")></span>
                                    <span class="project-search-main">
                                        <span class="project-search-title">{title}</span>
                                        {(!sub.trim().is_empty()).then(|| view! { <span class="project-search-sub">{sub}</span> })}
                                    </span>
                                    {opens_project_window.then(|| view! {
                                        <kbd class="action-shortcut project-window-shortcut">
                                            {if is_mac() { "⌘↵" } else { "Ctrl↵" }}" "{t(locale.get(), "command.hint.open_new_window")}
                                        </kbd>
                                    })}
                                </button>
                            }
                        }).collect_view()}
                    </div>
                    <div class="project-search-foot"><span><kbd>"↑↓"</kbd>{t(locale.get(), "command.hint.navigate")}</span><span><kbd>"↵"</kbd>{t(locale.get(), "command.hint.open")}</span><span><kbd>"⇧↵"</kbd>{t(locale.get(), "command.hint.attach")}</span><span><kbd>"esc"</kbd>{t(locale.get(), "command.hint.close")}</span><span class="palette-version">{concat!("v", env!("CARGO_PKG_VERSION"))}</span></div>
                </div>
            </div>
        })}
    }
}

#[component]
pub(crate) fn ActionPalette(
    open: RwSignal<bool>,
    on_action: Callback<&'static str>,
) -> impl IntoView {
    let locale = use_locale();
    let query = create_rw_signal(String::new());
    let active = create_rw_signal(0usize);
    create_effect(move |_| {
        if !open.get() {
            return;
        }
        query.set(String::new());
        active.set(0);
        let focus = Closure::once(|| {
            let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
                return;
            };
            let Some(input) = doc.get_element_by_id("action-palette-input") else {
                return;
            };
            let _ = input.dyn_ref::<web_sys::HtmlElement>().map(|el| el.focus());
        });
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                focus.as_ref().unchecked_ref(),
                0,
            );
        }
        focus.forget();
    });
    let actions = create_memo(move |_| {
        let loc = locale.get();
        let general = t(loc, "command.group.general").to_string();
        let navigate = t(loc, "command.group.navigate").to_string();
        let appearance = t(loc, "command.group.appearance").to_string();
        let entries = [
            (
                "scratch",
                "bubble",
                "command.scratch",
                general.clone(),
                "Ctrl/⌘ Shift N",
            ),
            (
                "new",
                "plus",
                "command.new_session",
                general.clone(),
                "Ctrl/⌘ N",
            ),
            (
                "search",
                "search",
                "command.search",
                general.clone(),
                "Ctrl/⌘ K",
            ),
            (
                "settings",
                "gear",
                "command.settings",
                general.clone(),
                "Ctrl/⌘ ,",
            ),
            (
                "import-codex",
                "download",
                "command.import_codex",
                general.clone(),
                "",
            ),
            (
                "import-claude",
                "download",
                "command.import_claude",
                general.clone(),
                "",
            ),
            (
                "check-updates",
                "gear",
                "command.check_updates",
                general.clone(),
                "",
            ),
            ("star-us", "star", "command.star_us", general.clone(), ""),
            (
                "project-settings",
                "gear",
                "command.project_settings",
                general.clone(),
                "",
            ),
            ("skills", "grid", "command.skills", general, ""),
            (
                "projects",
                "folder",
                "command.projects",
                navigate.clone(),
                "",
            ),
            ("library", "star", "command.library", navigate.clone(), ""),
            (
                "toggle-sidebar",
                "panel",
                "command.toggle_sidebar",
                navigate.clone(),
                "Ctrl/⌘ B",
            ),
            (
                "artifacts",
                "grid",
                "command.artifacts",
                navigate.clone(),
                "",
            ),
            ("notebook", "doc", "command.notebook", navigate.clone(), ""),
            ("files", "doc", "command.files", navigate.clone(), ""),
            (
                "provenance",
                "copy",
                "command.provenance",
                navigate.clone(),
                "",
            ),
            (
                "contexts",
                "server",
                "command.contexts",
                navigate.clone(),
                "",
            ),
            (
                "side-chat",
                "bubble",
                "command.side_chat",
                navigate.clone(),
                "",
            ),
            ("close-panel", "panel", "command.close_panel", navigate, ""),
            (
                "theme-light",
                "gear",
                "command.theme_light",
                appearance.clone(),
                "",
            ),
            (
                "theme-dark",
                "gear",
                "command.theme_dark",
                appearance.clone(),
                "",
            ),
            (
                "theme-system",
                "gear",
                "command.theme_system",
                appearance.clone(),
                "",
            ),
            (
                "font-ui-increase",
                "plus",
                "command.font_ui_increase",
                appearance.clone(),
                "",
            ),
            (
                "font-ui-decrease",
                "minus",
                "command.font_ui_decrease",
                appearance.clone(),
                "",
            ),
            (
                "font-code-increase",
                "plus",
                "command.font_code_increase",
                appearance.clone(),
                "",
            ),
            (
                "font-code-decrease",
                "minus",
                "command.font_code_decrease",
                appearance,
                "",
            ),
        ];
        let q = query.get().trim().to_lowercase();
        entries
            .into_iter()
            .filter_map(|(id, icon, key, group, shortcut)| {
                let title = t(loc, key).to_string();
                contains_search(&q, &[id, &title, &group]).then_some(CommandAction {
                    id,
                    icon,
                    title,
                    group,
                    shortcut,
                })
            })
            .collect::<Vec<_>>()
    });
    let run = Callback::new(move |index: usize| {
        let Some(action) = actions.get().get(index).cloned() else {
            return;
        };
        open.set(false);
        on_action.call(action.id);
    });
    view! {
        {move || open.get().then(|| view! {
            <div class="project-search-overlay action-palette-overlay" on:click=move |_| open.set(false)>
                <div class="project-search-dialog action-palette" role="dialog" aria-label="Command Palette"
                    on:click=|ev| ev.stop_propagation()>
                    <div class="project-search-input">
                        <span class="gi search"></span>
                        <input id="action-palette-input" type="text" inputmode="search" autofocus=true
                            autocomplete="off" autocorrect="off" autocapitalize="none" spellcheck="false"
                            placeholder=move || t(locale.get(), "command.placeholder")
                            prop:value=move || query.get()
                            on:input=move |ev| { query.set(event_target_value(&ev)); active.set(0); }
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if ime_composing(&ev) { return; }
                                let n = actions.get().len();
                                match ev.key().as_str() {
                                    "Escape" => { ev.prevent_default(); open.set(false); }
                                    "ArrowDown" => {
                                        ev.prevent_default();
                                        if n > 0 {
                                            let next = (active.get() + 1) % n;
                                            active.set(next);
                                            scroll_picker_item(".action-palette .project-search-row", next);
                                        }
                                    }
                                    "ArrowUp" => {
                                        ev.prevent_default();
                                        if n > 0 {
                                            let next = (active.get() + n - 1) % n;
                                            active.set(next);
                                            scroll_picker_item(".action-palette .project-search-row", next);
                                        }
                                    }
                                    "Enter" => { ev.prevent_default(); run.call(active.get()); }
                                    _ => {}
                                }
                            } />
                    </div>
                    <div class="project-search-results action-palette-results">
                        {move || {
                            let rows = actions.get();
                            rows.into_iter().enumerate().map(|(i, action)| {
                                let previous_group = (i > 0).then(|| actions.get().get(i - 1).map(|a| a.group.clone())).flatten();
                                let show_group = previous_group.as_deref() != Some(action.group.as_str());
                                view! {
                                    {show_group.then(|| view! { <div class="action-palette-group">{action.group.clone()}</div> })}
                                    <button type="button" class="project-search-row action-palette-row" class:active=move || active.get() == i
                                        on:mousemove=move |_| active.set(i)
                                        on:click=move |_| run.call(i)>
                                        <span class=format!("gi {}", action.icon)></span>
                                        <span class="project-search-main"><span class="project-search-title">{action.title}</span></span>
                                        {(!action.shortcut.is_empty()).then(|| view! { <kbd class="action-shortcut">{action.shortcut}</kbd> })}
                                    </button>
                                }
                            }).collect_view()
                        }}
                    </div>
                    <div class="project-search-foot"><span><kbd>"↑↓"</kbd>{t(locale.get(), "command.hint.navigate")}</span><span><kbd>"↵"</kbd>{t(locale.get(), "command.hint.run")}</span><span><kbd>"esc"</kbd>{t(locale.get(), "command.hint.close")}</span></div>
                </div>
            </div>
        })}
    }
}
