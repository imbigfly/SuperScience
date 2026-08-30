//! Shared WeKnora knowledge-base settings form and capability overlay.
//!
//! Used by Settings → Knowledge Base and the Efficiency Tools card popup so
//! both surfaces edit the same fields without a second form.

use crate::app_support::{compose_icon, js_error_text};
use crate::bindings::invoke_checked;
use crate::dto::{KnowledgeBaseSummary, KnowledgeConnectionTest, KnowledgeSettings};
use crate::i18n::{t, Locale};
use crate::text::{event_target_input, event_target_value};
use leptos::*;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::JsValue;

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

#[component]
pub(crate) fn KnowledgeSettingsForm(
    locale: RwSignal<Locale>,
    settings: RwSignal<KnowledgeSettings>,
    api_key: RwSignal<String>,
    msg: RwSignal<Option<(bool, String)>>,
    busy: RwSignal<bool>,
    bases: RwSignal<Vec<KnowledgeBaseSummary>>,
    on_save: Callback<()>,
    on_test: Callback<()>,
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
                <input data-testid="knowledge-base-ids"
                    prop:value=move || settings.get().weknora.knowledge_base_ids
                    placeholder="kb-00000001"
                    on:input=move |ev| settings.update(|current| {
                        current.weknora.knowledge_base_ids = event_target_input(&ev).value();
                    }) />
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
        {move || {
            let listed = bases.get();
            (!listed.is_empty()).then(move || view! {
                <ul class="knowledge-base-list" data-testid="knowledge-bases">
                    {listed.into_iter().map(|kb| view! {
                        <li><code>{kb.id}</code> <span>{kb.name}</span></li>
                    }).collect_view()}
                </ul>
            })
        }}
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

#[component]
pub(crate) fn KnowledgeSettingsOverlay(
    locale: RwSignal<Locale>,
    require_connection: bool,
    notice: RwSignal<Option<(bool, String)>>,
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
            settings.set(loaded);
            api_key.set(String::new());
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
                    on_save=save
                    on_test=test
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
