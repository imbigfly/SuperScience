//! Model & specialist settings domain: the form signals behind the Settings
//! "Models" and "Specialists" panes plus their save/validate/test handlers.
//! `App` constructs `ModelSettingsState` with the cross-domain signals it
//! depends on (profile list, settings, busy flag, locale) and passes the
//! handlers down to `SettingsView` as callbacks.

use super::*;
use crate::bindings::invoke_timeout;

/// Owned form state plus injected cross-domain wiring. `RwSignal` is `Copy`,
/// so the whole struct is `Copy` and can be captured by every handler closure.
#[derive(Clone, Copy)]
pub(crate) struct ModelSettingsState {
    pub(crate) model_form: RwSignal<Option<ModelForm>>,
    pub(crate) model_catalog_limits: RwSignal<Option<CatalogEntryDto>>,
    pub(crate) model_form_key: RwSignal<String>,
    pub(crate) model_form_msg: RwSignal<Option<(bool, String)>>,
    pub(crate) specialists: RwSignal<Vec<Specialist>>,
    pub(crate) specialist_form: RwSignal<Option<Specialist>>,
    // Cross-domain signals injected by `App`.
    pub(crate) models: RwSignal<Vec<ModelProfile>>,
    pub(crate) acp_agents: RwSignal<Vec<AcpAgentProfile>>,
    pub(crate) settings: RwSignal<Settings>,
    pub(crate) settings_busy: RwSignal<bool>,
    pub(crate) settings_message: RwSignal<Option<(bool, String)>>,
    pub(crate) locale: RwSignal<Locale>,
}

impl ModelSettingsState {
    pub(crate) fn new(
        models: RwSignal<Vec<ModelProfile>>,
        acp_agents: RwSignal<Vec<AcpAgentProfile>>,
        settings: RwSignal<Settings>,
        settings_busy: RwSignal<bool>,
        settings_message: RwSignal<Option<(bool, String)>>,
        locale: RwSignal<Locale>,
    ) -> Self {
        Self {
            model_form: create_rw_signal(None::<ModelForm>),
            model_catalog_limits: create_rw_signal(None::<CatalogEntryDto>),
            model_form_key: create_rw_signal(String::new()),
            model_form_msg: create_rw_signal(None::<(bool, String)>),
            specialists: create_rw_signal(vec![]),
            specialist_form: create_rw_signal(None::<Specialist>),
            models,
            acp_agents,
            settings,
            settings_busy,
            settings_message,
            locale,
        }
    }

    pub(crate) fn refresh_models(self) {
        let Self {
            models, acp_agents, ..
        } = self;
        spawn_local(async move {
            let v = invoke("list_models", JsValue::UNDEFINED).await;
            if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<ModelProfile>>(v) {
                models.set(list);
            }
            let v = invoke("list_acp_agents", JsValue::UNDEFINED).await;
            if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<AcpAgentProfile>>(v) {
                acp_agents.set(list);
            }
        })
    }

    pub(crate) fn refresh_specialists(self) {
        let specialists = self.specialists;
        spawn_local(async move {
            let v = invoke("list_specialists", JsValue::UNDEFINED).await;
            if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<Specialist>>(v) {
                specialists.set(list);
            }
        })
    }

    /// Persist a per-model reasoning-effort choice from the composer's picker.
    pub(crate) fn apply_model_effort(self, id: String, effort: String) {
        let models = self.models;
        let Some(profile) = models.get_untracked().into_iter().find(|m| m.id == id) else {
            return;
        };
        spawn_local(async move {
            let arg = to_value(&serde_json::json!({
                "profile": {
                    "id": profile.id,
                    "label": profile.label,
                    "provider": profile.provider,
                    "api_url": profile.api_url,
                    "model": profile.model,
                    "max_tokens": profile.max_tokens,
                    "context_window": profile.context_window,
                    "reasoning_effort": effort,
                    "supports_vision": profile.supports_vision,
                    "use_for_vision": profile.use_for_vision,
                    "use_for_image_generation": profile.use_for_image_generation,
                },
                // No key field: the backend keeps the stored key.
                "key": Option::<String>::None,
                "useForVision": profile.use_for_vision,
                "useForImageGeneration": profile.use_for_image_generation,
            }))
            .unwrap();
            match invoke_checked("save_model", arg).await {
                Ok(v) => {
                    if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<ModelProfile>>(v) {
                        models.set(list);
                    }
                }
                Err(err) => show_warning_toast(&js_error_text(err)),
            }
        });
    }

    pub(crate) fn save_model_form(self) {
        let Self {
            model_form,
            model_catalog_limits,
            model_form_key,
            model_form_msg,
            models,
            settings,
            settings_busy,
            locale,
            ..
        } = self;
        if settings_busy.get() {
            return;
        }
        let Some(form) = model_form.get() else {
            return;
        };
        let loc = locale.get();
        let key = model_form_key.get();
        let has_key = form
            .id
            .as_ref()
            .and_then(|id| {
                models
                    .get()
                    .iter()
                    .find(|m| &m.id == id)
                    .map(|m| m.has_api_key)
            })
            .unwrap_or(false);
        let cfg = model_form_to_settings(&form, has_key && key.is_empty());
        if let Some(err_key) = settings_required_error_key(&cfg, &key) {
            let err = t(loc, err_key);
            let text = tf(loc, "status.save_failed", &[("msg", &err)]);
            model_form_msg.set(Some((false, text)));
            return;
        }
        // A catalog-known model has a documented output ceiling; saving a
        // larger max_tokens only ever surfaces as a provider 400 mid-turn.
        if let Some(dto) = model_catalog_limits.get() {
            if form.max_tokens > dto.max_tokens {
                let text = tf(
                    loc,
                    "err.max_tokens_ceiling",
                    &[
                        ("model", form.model.trim()),
                        ("max", &dto.max_tokens.to_string()),
                    ],
                );
                model_form_msg.set(Some((false, text)));
                return;
            }
        }
        settings_busy.set(true);
        model_form_msg.set(Some((true, t(loc, "status.saving_settings").into())));
        let provider = provider_value(&form.provider);
        let profile = serde_json::json!({
            "id": form.id.clone().unwrap_or_default(),
            "label": form.label.trim(),
            "provider": provider,
            "api_url": form.api_url.trim(),
            "model": form.model.trim(),
            "max_tokens": form.max_tokens,
            "context_window": form.context_window,
            "reasoning_effort": form.reasoning_effort.trim(),
            "supports_vision": form.supports_vision,
            "use_for_vision": form.use_for_vision,
            "use_for_image_generation": form.use_for_image_generation,
        });
        let key_arg = if key.is_empty() { None } else { Some(key) };
        spawn_local(async move {
            let arg = to_value(&serde_json::json!({
                "profile": profile,
                "key": key_arg,
                "useForVision": form.use_for_vision,
                "useForImageGeneration": form.use_for_image_generation,
            }))
            .unwrap();
            match invoke_checked("save_model", arg).await {
                Ok(v) => {
                    if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<ModelProfile>>(v) {
                        models.set(list);
                    }
                    let v = invoke("get_settings", JsValue::UNDEFINED).await;
                    if let Ok(cfg) = serde_wasm_bindgen::from_value::<Settings>(v) {
                        settings.set(normalized_settings(cfg));
                    }
                    model_form.set(None);
                    model_form_key.set(String::new());
                    model_form_msg.set(Some((true, t(loc, "status.settings_saved").into())));
                }
                Err(err) => {
                    model_form_msg.set(Some((false, localize_backend(loc, &js_error_text(err)))));
                }
            }
            settings_busy.set(false);
        });
    }

    pub(crate) fn validate_model_form(self) {
        let Self {
            model_form,
            model_form_key,
            model_form_msg,
            models,
            settings_busy,
            locale,
            ..
        } = self;
        if settings_busy.get() {
            return;
        }
        let Some(form) = model_form.get() else {
            return;
        };
        let loc = locale.get();
        let key = model_form_key.get();
        let has_key = models
            .get()
            .iter()
            .find(|m| Some(m.id.as_str()) == form.id.as_deref())
            .map(|m| m.has_api_key)
            .unwrap_or(false);
        let cfg = model_form_to_settings(&form, has_key);
        if let Some(err_key) = settings_required_error_key(&cfg, &key) {
            let err = t(loc, err_key);
            model_form_msg.set(Some((
                false,
                tf(loc, "status.validation_failed", &[("msg", &err)]),
            )));
            return;
        }
        settings_busy.set(true);
        model_form_msg.set(Some((true, t(loc, "status.validating").into())));
        // The backend probes with a test image when "supports images" is on,
        // so both outcomes say which probe ran — a checked box was never
        // proof that the model takes images.
        let vision = cfg.supports_vision;
        spawn_local(async move {
            let res = invoke_timeout(
                "validate_settings",
                to_value(&serde_json::json!({
                    "settings": cfg,
                    "key": key,
                    "profileId": form.id.clone(),
                }))
                .unwrap(),
                35_000,
            )
            .await;
            match res {
                Ok(v) => {
                    let raw = v
                        .as_string()
                        .unwrap_or_else(|| t(loc, "status.validation_succeeded").into());
                    let mut msg = localize_backend(loc, &raw);
                    if vision {
                        msg.push_str(&t(loc, "status.vision_ok"));
                    }
                    model_form_msg.set(Some((true, msg)));
                }
                Err(err) => {
                    let mut msg = tf(
                        loc,
                        "status.validation_failed",
                        &[("msg", &localize_backend(loc, &js_error_text(err)))],
                    );
                    if vision {
                        msg.push_str(&t(loc, "err.vision_probe_failed"));
                    }
                    model_form_msg.set(Some((false, msg)));
                }
            }
            settings_busy.set(false);
        });
    }

    pub(crate) fn test_reviewer_form(self) {
        let Self {
            specialist_form,
            model_form_msg,
            settings_busy,
            locale,
            ..
        } = self;
        let Some(spec) = specialist_form.get() else {
            return;
        };
        if spec.id != "reviewer" || settings_busy.get() {
            return;
        }
        let loc = locale.get();
        settings_busy.set(true);
        model_form_msg.set(Some((true, t(loc, "specialists.reviewer.testing").into())));
        spawn_local(async move {
            let result = invoke_timeout(
                "test_reviewer_backend",
                to_value(&serde_json::json!({ "reviewer": spec })).unwrap(),
                120_000,
            )
            .await;
            match result {
                Ok(value) => {
                    match serde_wasm_bindgen::from_value::<ReviewerBackendTestResult>(value) {
                        Ok(result) => {
                            let backend = match result.backend.as_str() {
                                "acp_agent" => "ACP",
                                "http_model" => "HTTP",
                                other => other,
                            };
                            let headline = tf(
                                loc,
                                "specialists.reviewer.test_ok",
                                &[
                                    ("backend", backend),
                                    ("model", &result.model),
                                    ("status", &result.status),
                                ],
                            );
                            model_form_msg.set(Some((
                                true,
                                if result.summary.trim().is_empty() {
                                    headline
                                } else {
                                    format!("{headline} {}", result.summary.trim())
                                },
                            )));
                        }
                        Err(error) => model_form_msg.set(Some((false, error.to_string()))),
                    }
                }
                Err(error) => model_form_msg.set(Some((
                    false,
                    tf(
                        loc,
                        "specialists.reviewer.test_failed",
                        &[("msg", &localize_backend(loc, &js_error_text(error)))],
                    ),
                ))),
            }
            settings_busy.set(false);
        });
    }

    pub(crate) fn save_specialist_form(self) {
        let Self {
            specialists,
            specialist_form,
            model_form_msg,
            settings_busy,
            settings_message,
            locale,
            ..
        } = self;
        let Some(spec) = specialist_form.get() else {
            return;
        };
        let loc = locale.get();
        if spec.name.trim().is_empty() {
            model_form_msg.set(Some((false, t(loc, "specialists.name_required").into())));
            return;
        }
        let saved_id = spec.id.clone();
        let keep_open = saved_id == "reviewer";
        settings_busy.set(true);
        model_form_msg.set(Some((true, t(loc, "status.saving_settings").into())));
        spawn_local(async move {
            let args = to_value(&serde_json::json!({ "spec": spec })).unwrap();
            match invoke_checked("save_specialist_cmd", args).await {
                Ok(value) => match serde_wasm_bindgen::from_value::<Vec<Specialist>>(value) {
                    Ok(value) => {
                        let saved = value.iter().find(|item| item.id == saved_id).cloned();
                        specialists.set(value);
                        if keep_open {
                            specialist_form.set(saved);
                            model_form_msg.set(Some((true, t(loc, "specialists.saved").into())));
                        } else {
                            specialist_form.set(None);
                            settings_message.set(Some((true, t(loc, "specialists.saved").into())));
                        }
                    }
                    Err(error) => model_form_msg.set(Some((false, error.to_string()))),
                },
                Err(error) => model_form_msg.set(Some((false, js_error_text(error)))),
            }
            settings_busy.set(false);
        });
    }

    pub(crate) fn remove_specialist(self, id: String) {
        let Self {
            specialists,
            settings_message,
            ..
        } = self;
        spawn_local(async move {
            let args = to_value(&serde_json::json!({ "id": id })).unwrap();
            match invoke_checked("remove_specialist", args).await {
                Ok(value) => match serde_wasm_bindgen::from_value::<Vec<Specialist>>(value) {
                    Ok(value) => specialists.set(value),
                    Err(error) => settings_message.set(Some((false, error.to_string()))),
                },
                Err(error) => settings_message.set(Some((false, js_error_text(error)))),
            }
        });
    }
}
