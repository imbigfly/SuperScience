//! Shared WeKnora knowledge-base settings form and capability overlay.
//!
//! Used by Settings → Knowledge Base and the Efficiency Tools card popup so
//! both surfaces edit the same fields without a second form.

use crate::app_support::{compose_icon, js_error_text};
use crate::bindings::invoke_checked;
use crate::dto::{KnowledgeBaseSummary, KnowledgeConnectionTest, KnowledgeSettings};
use crate::i18n::{t, tf, Locale};
use crate::text::event_target_input;
use leptos::html;
use leptos::*;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::JsValue;

/// Split the stored comma/whitespace list into ids for checkbox state.
fn parse_kb_ids(raw: &str) -> Vec<String> {
    raw.split(|c: char| c == ',' || c.is_whitespace())
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

/// Add or remove one id, keeping the remaining order and comma separation.
fn toggle_kb_id(raw: &str, id: &str, on: bool) -> String {
    let mut ids = parse_kb_ids(raw);
    ids.retain(|existing| existing != id);
    if on {
        ids.push(id.to_string());
    }
    ids.join(",")
}

pub(crate) fn knowledge_settings_configured(settings: &KnowledgeSettings) -> bool {
    settings.provider.trim() == "weknora"
        && (settings.weknora.has_api_key || !settings.weknora.api_key.trim().is_empty())
        && !settings.weknora.knowledge_base_ids.trim().is_empty()
}

pub(crate) fn prepare_knowledge_settings(
    mut settings: KnowledgeSettings,
    api_key: String,
) -> KnowledgeSettings {
    if settings.provider.trim().is_empty() {
        settings.provider = "weknora".into();
    }
    settings.weknora.api_key = api_key;
    settings
}

pub(crate) async fn load_knowledge_settings() -> Result<KnowledgeSettings, String> {
    let value = invoke_checked("get_knowledge_settings", JsValue::UNDEFINED)
        .await
        .map_err(js_error_text)?;
    serde_wasm_bindgen::from_value::<KnowledgeSettings>(value).map_err(|error| error.to_string())
}

pub(crate) async fn persist_knowledge_settings(
    settings: KnowledgeSettings,
    api_key: String,
) -> Result<KnowledgeSettings, String> {
    let next = prepare_knowledge_settings(settings, api_key);
    let arg =
        to_value(&serde_json::json!({ "settings": next })).map_err(|error| error.to_string())?;
    let value = invoke_checked("set_knowledge_settings", arg)
        .await
        .map_err(js_error_text)?;
    serde_wasm_bindgen::from_value::<KnowledgeSettings>(value).map_err(|error| error.to_string())
}

pub(crate) async fn test_knowledge_settings(
    settings: KnowledgeSettings,
    api_key: String,
) -> Result<KnowledgeConnectionTest, String> {
    let next = prepare_knowledge_settings(settings, api_key);
    let arg =
        to_value(&serde_json::json!({ "settings": next })).map_err(|error| error.to_string())?;
    let value = invoke_checked("test_knowledge_connection", arg)
        .await
        .map_err(js_error_text)?;
    serde_wasm_bindgen::from_value::<KnowledgeConnectionTest>(value)
        .map_err(|error| error.to_string())
}

pub(crate) async fn probe_saved_knowledge_connection() -> Result<KnowledgeConnectionTest, String> {
    let value = invoke_checked("test_knowledge_connection", JsValue::UNDEFINED)
        .await
        .map_err(js_error_text)?;
    serde_wasm_bindgen::from_value::<KnowledgeConnectionTest>(value)
        .map_err(|error| error.to_string())
}

pub(crate) async fn probe_knowledge_ready() -> Result<KnowledgeConnectionTest, String> {
    let settings = load_knowledge_settings().await?;
    if !knowledge_settings_configured(&settings) {
        return Ok(KnowledgeConnectionTest {
            ok: false,
            message: String::new(),
            knowledge_bases: Vec::new(),
        });
    }
    probe_saved_knowledge_connection().await
}

/// Fetch the knowledge bases reachable with the given (or saved) settings, so
/// the dropdown can list what the user is editing rather than only what was
/// last saved.
pub(crate) async fn list_knowledge_settings_bases(
    settings: KnowledgeSettings,
    api_key: String,
) -> Result<Vec<KnowledgeBaseSummary>, String> {
    let next = prepare_knowledge_settings(settings, api_key);
    let arg =
        to_value(&serde_json::json!({ "settings": next })).map_err(|error| error.to_string())?;
    let value = invoke_checked("list_knowledge_bases", arg)
        .await
        .map_err(js_error_text)?;
    serde_wasm_bindgen::from_value::<Vec<KnowledgeBaseSummary>>(value)
        .map_err(|error| error.to_string())
}

#[component]
pub(crate) fn KnowledgeSettingsForm(
    locale: RwSignal<Locale>,
    settings: RwSignal<KnowledgeSettings>,
    api_key: RwSignal<String>,
    msg: RwSignal<Option<(bool, String)>>,
    busy: RwSignal<bool>,
    bases: RwSignal<Vec<KnowledgeBaseSummary>>,
    menu_open: RwSignal<bool>,
    on_save: Callback<()>,
    on_test: Callback<()>,
    on_refresh: Callback<()>,
) -> impl IntoView {
    view! {
        {move || msg.get().map(|(ok, text)| view! {
            <div class="settings-status" class:ok=ok class:fail=move || !ok
                data-testid="knowledge-settings-status">{text}</div>
        })}
        <div class="settings-form-grid">
            <label>{move || t(locale.get(), "knowledge.provider")}
                <select data-testid="knowledge-provider"
                    prop:value=move || {
                        let provider = settings.get().provider;
                        if provider.is_empty() { "weknora".into() } else { provider }
                    }
                    on:change=move |ev| settings.update(|current| {
                        current.provider = event_target_value(&ev);
                    })>
                    <option value="weknora">{move || t(locale.get(), "knowledge.provider.weknora")}</option>
                </select>
            </label>
            <label class="span-2">{move || t(locale.get(), "knowledge.weknora.base_url")}
                <input data-testid="knowledge-base-url" type="url"
                    prop:value=move || settings.get().weknora.base_url
                    placeholder="http://localhost:8080/api/v1"
                    on:input=move |ev| settings.update(|current| {
                        current.weknora.base_url = event_target_input(&ev).value();
                    }) />
                <span class="settings-field-hint">{move || t(locale.get(), "knowledge.weknora.base_url_hint")}</span>
            </label>
            <label class="span-2">{move || t(locale.get(), "knowledge.weknora.api_key")}
                <input data-testid="knowledge-api-key" type="password"
                    prop:value=move || api_key.get()
                    placeholder=move || if settings.get().weknora.has_api_key {
                        t(locale.get(), "settings.key_stored")
                    } else {
                        t(locale.get(), "knowledge.weknora.api_key_placeholder")
                    }
                    on:input=move |ev| api_key.set(event_target_input(&ev).value()) />
                <span class="settings-field-hint">{move || t(locale.get(), "knowledge.weknora.api_key_hint")}</span>
            </label>
            <label class="span-2">{move || t(locale.get(), "knowledge.weknora.kb_ids")}
                <KnowledgeBasePicker
                    locale=locale
                    settings=settings
                    bases=bases
                    busy=busy
                    menu_open=menu_open
                    on_refresh=on_refresh
                />
                <span class="settings-field-hint">{move || t(locale.get(), "knowledge.weknora.kb_ids_hint")}</span>
            </label>
            <label>{move || t(locale.get(), "knowledge.weknora.match_count")}
                <input data-testid="knowledge-match-count" type="number" min="1" max="32"
                    prop:value=move || settings.get().weknora.match_count.to_string()
                    on:input=move |ev| settings.update(|current| {
                        current.weknora.match_count = event_target_input(&ev).value().parse().unwrap_or(8);
                    }) />
                <span class="settings-field-hint">{move || t(locale.get(), "knowledge.weknora.match_count_hint")}</span>
            </label>
        </div>
        <p class="settings-field-hint">{move || t(locale.get(), "knowledge.secret_note")}</p>
        <div class="row settings-footer">
            <button type="button" data-testid="knowledge-test"
                disabled=move || busy.get()
                on:click=move |_| on_test.call(())>
                {move || t(locale.get(), "knowledge.test")}
            </button>
            <button type="button" class="primary" data-testid="knowledge-save"
                disabled=move || busy.get()
                on:click=move |_| on_save.call(())>
                {move || t(locale.get(), "knowledge.save")}
            </button>
        </div>
    }
}

/// 多选下拉：触发器显示已选名称（或占位文案），菜单是勾选列表，直接写回
/// `knowledge_base_ids`，因此保存表单时选择状态随其它字段一起持久化。
///
/// `menu_open` 由父级 surface 持有，这样窗口级 Escape 栈可以先关菜单再关弹窗。
#[component]
fn KnowledgeBasePicker(
    locale: RwSignal<Locale>,
    settings: RwSignal<KnowledgeSettings>,
    bases: RwSignal<Vec<KnowledgeBaseSummary>>,
    busy: RwSignal<bool>,
    menu_open: RwSignal<bool>,
    on_refresh: Callback<()>,
) -> impl IntoView {
    let open = menu_open;
    let picker = create_node_ref::<html::Div>();
    let trigger = create_node_ref::<html::Button>();
    on_cleanup(move || open.set(false));
    // 点击触发器/菜单之外即关闭。
    window_event_listener(ev::mousedown, move |event| {
        use wasm_bindgen::JsCast;
        if !open.get_untracked() {
            return;
        }
        let inside = picker
            .get_untracked()
            .zip(
                event
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Node>().ok()),
            )
            .is_some_and(|(root, target)| root.contains(Some(&target)));
        if !inside {
            open.set(false);
        }
    });
    // 菜单用 fixed 定位跟随触发器，避免被设置面板的 overflow-y 裁剪。前提是
    // 触发器和菜单之间没有 transform/filter 祖先：一旦有，fixed 会改以那个祖先
    // 为包含块，坐标按祖先算、还会被祖先的 overflow 裁掉（.modal 的入场动画曾
    // 如此，见 visual-polish.spec.ts 的 floating menu 用例）。
    let menu_style = move || {
        let Some(rect) = trigger
            .get_untracked()
            .map(|el| el.get_bounding_client_rect())
        else {
            return String::new();
        };
        format!(
            "left:{left}px;top:{top}px;width:{width}px;",
            left = rect.left(),
            top = rect.bottom() + 4.0,
            width = rect.width(),
        )
    };

    let trigger_label = move || {
        let selected = parse_kb_ids(&settings.get().weknora.knowledge_base_ids);
        if selected.is_empty() {
            return t(locale.get(), "knowledge.weknora.kb_placeholder").to_string();
        }
        let listed = bases.get();
        let names: Vec<String> = selected
            .iter()
            .map(|id| {
                listed
                    .iter()
                    .find(|kb| &kb.id == id)
                    .map(|kb| {
                        if kb.name.trim().is_empty() {
                            id.clone()
                        } else {
                            kb.name.clone()
                        }
                    })
                    .unwrap_or_else(|| id.clone())
            })
            .collect();
        if names.len() <= 2 {
            names.join("、")
        } else {
            tf(
                locale.get(),
                "knowledge.weknora.kb_selected",
                &[("count", &names.len().to_string())],
            )
        }
    };

    view! {
        <div class="knowledge-kb-select" node_ref=picker>
            <div class="knowledge-kb-select-row">
                <button type="button"
                    class="knowledge-kb-trigger"
                    data-testid="knowledge-base-select"
                    node_ref=trigger
                    attr:data-selected-ids=move || settings.get().weknora.knowledge_base_ids
                    aria-haspopup="listbox"
                    aria-expanded=move || open.get().to_string()
                    class:active=move || open.get()
                    on:click=move |_| open.update(|value| *value = !*value)>
                    <span class="knowledge-kb-trigger-label"
                        class:placeholder=move || parse_kb_ids(&settings.get().weknora.knowledge_base_ids).is_empty()>
                        {trigger_label}
                    </span>
                    <span class="knowledge-kb-caret" aria-hidden="true">{compose_icon("chevron-down")}</span>
                </button>
                <button type="button" class="knowledge-kb-refresh"
                    data-testid="knowledge-base-refresh"
                    disabled=move || busy.get()
                    title=move || t(locale.get(), "knowledge.weknora.kb_refresh")
                    aria-label=move || t(locale.get(), "knowledge.weknora.kb_refresh")
                    on:click=move |_| on_refresh.call(())>
                    {compose_icon("sync")}
                </button>
            </div>
            {move || open.get().then(|| view! {
                <div class="knowledge-kb-menu" role="listbox" aria-multiselectable="true"
                    data-testid="knowledge-base-menu" style=menu_style>
                    {move || {
                        let listed = bases.get();
                        if listed.is_empty() {
                            return view! {
                                <p class="knowledge-kb-empty" data-testid="knowledge-bases-empty">
                                    {move || t(locale.get(), "knowledge.weknora.kb_empty")}
                                </p>
                            }.into_view();
                        }
                        listed.into_iter().map(|kb| {
                            let option_id = kb.id.clone();
                            let toggle_id = kb.id.clone();
                            let checked = parse_kb_ids(&settings.get().weknora.knowledge_base_ids)
                                .contains(&kb.id);
                            view! {
                                <button type="button"
                                    class="knowledge-kb-option"
                                    role="option"
                                    aria-selected=checked.to_string()
                                    data-testid="knowledge-base-option"
                                    data-kb-id=option_id.clone()
                                    class:selected=checked
                                    on:click=move |_| {
                                        settings.update(|current| {
                                            current.weknora.knowledge_base_ids = toggle_kb_id(
                                                &current.weknora.knowledge_base_ids,
                                                &toggle_id,
                                                !checked,
                                            );
                                        });
                                    }>
                                    <span class="knowledge-kb-check" aria-hidden="true">
                                        {(checked).then(|| compose_icon("check"))}
                                    </span>
                                    <span class="knowledge-kb-option-text">
                                        <span class="knowledge-kb-option-name">{kb.name.clone()}</span>
                                        <code class="knowledge-kb-option-id">{kb.id.clone()}</code>
                                    </span>
                                </button>
                            }
                        }).collect_view()
                    }}
                </div>
            })}
        </div>
    }
}

#[component]
pub(crate) fn KnowledgeSettingsOverlay(
    locale: RwSignal<Locale>,
    require_connection: bool,
    notice: RwSignal<Option<(bool, String)>>,
    menu_open: RwSignal<bool>,
    on_close: Callback<()>,
    on_connected: Callback<()>,
) -> impl IntoView {
    let settings = create_rw_signal(KnowledgeSettings::default());
    let api_key = create_rw_signal(String::new());
    let msg = create_rw_signal(notice.get_untracked());
    let busy = create_rw_signal(false);
    let bases = create_rw_signal(Vec::<KnowledgeBaseSummary>::new());
    spawn_local(async move {
        if let Ok(loaded) = load_knowledge_settings().await {
            // Best-effort fill of the picker so saved ids render as names
            // without the user pressing refresh first.
            let listed = list_knowledge_settings_bases(loaded.clone(), String::new()).await;
            settings.set(loaded);
            api_key.set(String::new());
            if let Ok(listed) = listed {
                bases.set(listed);
            }
        }
    });
    let save = Callback::new(move |_: ()| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match persist_knowledge_settings(settings.get_untracked(), api_key.get_untracked())
                .await
            {
                Ok(saved) => {
                    settings.set(saved.clone());
                    api_key.set(String::new());
                    if require_connection {
                        match test_knowledge_settings(saved, String::new()).await {
                            Ok(result) => {
                                bases.set(result.knowledge_bases.clone());
                                if result.ok {
                                    msg.set(None);
                                    on_connected.call(());
                                } else {
                                    let text = if result.message.trim().is_empty() {
                                        t(
                                            locale.get_untracked(),
                                            "caps.tile.knowledge.settings.required",
                                        )
                                        .into()
                                    } else {
                                        result.message
                                    };
                                    msg.set(Some((false, text)));
                                }
                            }
                            Err(err) => msg.set(Some((false, err))),
                        }
                    } else {
                        msg.set(Some((
                            true,
                            t(locale.get_untracked(), "knowledge.saved").into(),
                        )));
                        on_close.call(());
                    }
                }
                Err(err) => msg.set(Some((false, err))),
            }
            busy.set(false);
        });
    });
    let test = Callback::new(move |_: ()| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match test_knowledge_settings(settings.get_untracked(), api_key.get_untracked()).await {
                Ok(result) => {
                    bases.set(result.knowledge_bases.clone());
                    msg.set(Some((result.ok, result.message)));
                }
                Err(err) => msg.set(Some((false, err))),
            }
            busy.set(false);
        });
    });
    let refresh = Callback::new(move |_: ()| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match list_knowledge_settings_bases(settings.get_untracked(), api_key.get_untracked())
                .await
            {
                Ok(listed) => {
                    let count = listed.len();
                    bases.set(listed);
                    msg.set(Some((
                        true,
                        tf(
                            locale.get_untracked(),
                            "knowledge.weknora.kb_refreshed",
                            &[("count", &count.to_string())],
                        ),
                    )));
                }
                Err(err) => msg.set(Some((false, err))),
            }
            busy.set(false);
        });
    });
    view! {
        <div class="overlay" data-testid="knowledge-settings-overlay"
            on:click=move |_| on_close.call(())>
            <div class="modal cap-info-modal knowledge-settings-modal" role="dialog" aria-modal="true"
                aria-labelledby="knowledge-settings-title"
                on:click=move |ev| ev.stop_propagation()>
                <div class="ps-head">
                    <h2 id="knowledge-settings-title">
                        {move || t(locale.get(), "caps.tile.knowledge.settings.title")}
                    </h2>
                    <button type="button" class="ps-close"
                        aria-label=move || t(locale.get(), "caps.tile.knowledge.settings.close")
                        on:click=move |_| on_close.call(())>
                        {compose_icon("close")}
                    </button>
                </div>
                <p class="cap-info-lead">{move || t(locale.get(), "knowledge.desc")}</p>
                <KnowledgeSettingsForm
                    locale=locale
                    settings=settings
                    api_key=api_key
                    msg=msg
                    busy=busy
                    bases=bases
                    menu_open=menu_open
                    on_save=save
                    on_test=test
                    on_refresh=refresh
                />
                <div class="row">
                    <button type="button"
                        on:click=move |_| on_close.call(())>
                        {move || t(locale.get(), "caps.tile.knowledge.settings.close")}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::WeKnoraSettings;

    fn settings(provider: &str, has_key: bool, ids: &str) -> KnowledgeSettings {
        KnowledgeSettings {
            provider: provider.into(),
            weknora: WeKnoraSettings {
                has_api_key: has_key,
                knowledge_base_ids: ids.into(),
                ..WeKnoraSettings::default()
            },
        }
    }

    #[test]
    fn configured_needs_provider_key_and_ids() {
        assert!(!knowledge_settings_configured(&KnowledgeSettings::default()));
        assert!(!knowledge_settings_configured(&settings(
            "weknora", true, ""
        )));
        assert!(!knowledge_settings_configured(&settings(
            "weknora", false, "kb-1"
        )));
        assert!(!knowledge_settings_configured(&settings("", true, "kb-1")));
        assert!(knowledge_settings_configured(&settings(
            "weknora", true, "kb-1"
        )));
        let typed = KnowledgeSettings {
            provider: "weknora".into(),
            weknora: WeKnoraSettings {
                api_key: "sk-test".into(),
                knowledge_base_ids: "kb-1".into(),
                ..WeKnoraSettings::default()
            },
        };
        assert!(knowledge_settings_configured(&typed));
    }

    #[test]
    fn prepare_defaults_empty_provider_to_weknora() {
        let next = prepare_knowledge_settings(KnowledgeSettings::default(), "sk".into());
        assert_eq!(next.provider, "weknora");
        assert_eq!(next.weknora.api_key, "sk");
    }
}
