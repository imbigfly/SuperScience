use crate::app_support::{
    compose_icon, copy_text, js_error_text, parse_redact_keywords, redact_text,
    refresh_execution_contexts, refresh_runs, refresh_runtimes, share_cards_from_highlights,
    share_fallback_caption, share_html_document, share_key_cards, share_png_payload, share_png_row,
    share_social_pack_text, show_toast, ShareCardPreview, ShareExportFormat, ShareHtmlRow,
    ShareMessage, ShareRole,
};
use crate::bindings::{copy_share_pack, invoke_checked, open_external_url, render_share_png};
use crate::dto::*;
use crate::i18n::{localize_backend, t, tf, Locale};
use crate::text::{dom_value, event_target_value, file_kind, format_bytes};
use leptos::*;
use serde_wasm_bindgen::to_value;
use std::rc::Rc;

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

fn share_role_key(role: ShareRole) -> &'static str {
    match role {
        ShareRole::User => "share.role_user",
        ShareRole::Assistant => "share.role_assistant",
        ShareRole::Thinking => "share.role_thinking",
    }
}

fn share_stamp() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .as_deref()
        .and_then(|iso| iso.get(..10))
        .unwrap_or("")
        .to_string()
}

fn selected_share_messages<'a>(
    messages: &'a [ShareMessage],
    keywords: &str,
) -> (Vec<&'a ShareMessage>, Vec<String>) {
    let redact = parse_redact_keywords(keywords);
    let selected = messages.iter().filter(|message| message.selected).collect();
    (selected, redact)
}

fn share_png_rows(loc: Locale, selected: &[&ShareMessage], redact: &[String]) -> Vec<serde_json::Value> {
    selected
        .iter()
        .map(|message| {
            share_png_row(
                message.role,
                &t(loc, share_role_key(message.role)),
                &redact_text(&message.text, redact),
            )
        })
        .collect()
}

fn default_share_platform(locale: Locale) -> ShareSocialPlatform {
    if locale == Locale::Zh {
        ShareSocialPlatform::Xiaohongshu
    } else {
        ShareSocialPlatform::Twitter
    }
}

fn selected_draft_indexes(messages: &[ShareMessage]) -> Vec<usize> {
    messages
        .iter()
        .enumerate()
        .filter(|(_, message)| message.selected)
        .map(|(index, _)| index)
        .collect()
}

/// `/share` preview dialog. Opening it prepares paste-ready copy and a few
/// key screenshots so the user does not have to write or crop anything.
/// `draft` is root-owned (`None` = closed) so the app-level Escape stack can
/// dismiss it in visual order.
#[component]
pub(super) fn ShareOverlay(
    locale: RwSignal<Locale>,
    draft: RwSignal<Option<Vec<ShareMessage>>>,
    session_id: RwSignal<Option<String>>,
) -> impl IntoView {
    let keywords = create_rw_signal(String::new());
    let busy = create_rw_signal(false);
    let generating = create_rw_signal(false);
    let error = create_rw_signal(None::<String>);
    let format = create_rw_signal(ShareExportFormat::Social);
    let platform = create_rw_signal(ShareSocialPlatform::Xiaohongshu);
    let social_copy = create_rw_signal(None::<ShareSocialCopy>);
    let selected_variant = create_rw_signal(0usize);
    let edit_body = create_rw_signal(String::new());
    let copy_dirty = create_rw_signal(false);
    let cards = create_rw_signal(Vec::<ShareCardPreview>::new());
    let selected_card = create_rw_signal(0usize);
    let generate_seq = create_rw_signal(0u32);
    let card_seq = create_rw_signal(0u32);
    // Render from the open flag, not the editable draft, so checkbox toggles
    // and keyword input do not rebuild the dialog DOM and drop focus.
    let open = create_memo(move |_| draft.with(|value| value.is_some()));
    let render_cards = Rc::new(move |previews: Vec<ShareCardPreview>| {
        let next = card_seq.get_untracked().wrapping_add(1);
        card_seq.set(next);
        selected_card.set(0);
        cards.set(previews.clone());
        let messages = draft.get_untracked().unwrap_or_default();
        let redact = parse_redact_keywords(&keywords.get_untracked());
        let loc = locale.get_untracked();
        let stamp = share_stamp();
        let footer = t(loc, "share.image_footer");
        let jobs: Vec<(usize, String)> = previews
            .iter()
            .enumerate()
            .map(|(index, card)| {
                let rows: Vec<serde_json::Value> = card
                    .indexes
                    .iter()
                    .filter_map(|row| messages.get(*row))
                    .map(|message| {
                        share_png_row(
                            message.role,
                            &t(loc, share_role_key(message.role)),
                            &redact_text(&message.text, &redact),
                        )
                    })
                    .collect();
                let title = if card.title.is_empty() {
                    "wisp-science"
                } else {
                    card.title.as_str()
                };
                (
                    index,
                    share_png_payload(title, &stamp, &footer, &rows),
                )
            })
            .collect();
        spawn_local(async move {
            for (index, payload) in jobs {
                if card_seq.get_untracked() != next || draft.get_untracked().is_none() {
                    return;
                }
                if let Some(png) = render_share_png(&payload)
                    .await
                    .ok()
                    .and_then(|value| value.as_string())
                    .filter(|png| !png.is_empty())
                {
                    cards.update(|list| {
                        if let Some(card) = list.get_mut(index) {
                            card.png_base64 = Some(png);
                        }
                    });
                }
            }
        });
    });
    let request_copy = {
        let render_cards = Rc::clone(&render_cards);
        Rc::new(move || {
        let loc = locale.get_untracked();
        let messages = draft.get_untracked().unwrap_or_default();
        let (selected, redact) = selected_share_messages(&messages, &keywords.get_untracked());
        if selected.is_empty() {
            error.set(Some(t(loc, "share.none_selected")));
            return;
        }
        let Some(frame_id) = session_id.get_untracked().filter(|id| !id.is_empty()) else {
            if !copy_dirty.get_untracked() {
                edit_body.set(share_fallback_caption(&messages, platform.get_untracked().body_limit()));
            }
            return;
        };
        let seq = generate_seq.get_untracked().wrapping_add(1);
        generate_seq.set(seq);
        generating.set(true);
        error.set(None);
        let platform_id = platform.get_untracked();
        let selected_indexes = selected_draft_indexes(&messages);
        let rows: Vec<serde_json::Value> = selected
            .iter()
            .map(|message| {
                serde_json::json!({
                    "role": message.role.tag(),
                    "text": redact_text(&message.text, &redact),
                })
            })
            .collect();
        let locale_tag = if loc == Locale::Zh { "zh" } else { "en" };
        let args = to_value(&serde_json::json!({
            "sessionId": frame_id,
            "platform": platform_id.as_str(),
            "locale": locale_tag,
            "messages": rows,
        }))
        .unwrap();
        let render_cards = Rc::clone(&render_cards);
        let messages = messages.clone();
        spawn_local(async move {
            let result = invoke_checked("generate_share_social_copy", args).await;
            if generate_seq.get_untracked() != seq || draft.get_untracked().is_none() {
                return;
            }
            generating.set(false);
            match result {
                Ok(value) => match serde_wasm_bindgen::from_value::<ShareSocialCopy>(value) {
                    Ok(copy) if !copy.variants.is_empty() => {
                        if !copy_dirty.get_untracked() {
                            let pack = share_social_pack_text(&copy.variants[0]);
                            selected_variant.set(0);
                            edit_body.set(pack);
                        }
                        let mapped = share_cards_from_highlights(
                            &messages,
                            &selected_indexes,
                            &copy.highlights,
                        );
                        if !mapped.is_empty() {
                            render_cards(
                                mapped
                                    .into_iter()
                                    .map(ShareCardPreview::from_spec)
                                    .collect(),
                            );
                        }
                        social_copy.set(Some(copy));
                    }
                    _ => {
                        if !copy_dirty.get_untracked()
                            && edit_body.get_untracked().trim().is_empty()
                        {
                            edit_body.set(share_fallback_caption(
                                &messages,
                                platform_id.body_limit(),
                            ));
                        }
                    }
                },
                Err(value) => {
                    if !copy_dirty.get_untracked()
                        && edit_body.get_untracked().trim().is_empty()
                    {
                        edit_body.set(share_fallback_caption(
                            &messages,
                            platform_id.body_limit(),
                        ));
                    }
                    error.set(Some(localize_backend(
                        locale.get_untracked(),
                        &js_error_text(value),
                    )));
                }
            }
        });
        })
    };
    let prime_share = {
        let render_cards = Rc::clone(&render_cards);
        let request_copy = Rc::clone(&request_copy);
        move || {
        let messages = draft.get_untracked().unwrap_or_default();
        let next_platform = default_share_platform(locale.get_untracked());
        platform.set(next_platform);
        format.set(ShareExportFormat::Social);
        social_copy.set(None);
        selected_variant.set(0);
        copy_dirty.set(false);
        error.set(None);
        edit_body.set(share_fallback_caption(&messages, next_platform.body_limit()));
        render_cards(
            share_key_cards(&messages)
                .into_iter()
                .map(ShareCardPreview::from_spec)
                .collect(),
        );
        request_copy();
        }
    };
    create_effect(move |previous: Option<bool>| {
        let now = open.get();
        if now && previous != Some(true) {
            keywords.set(String::new());
            busy.set(false);
            generating.set(false);
            prime_share();
        }
        now
    });
    let selected_count = create_memo(move |_| {
        draft.with(|value| {
            value
                .as_ref()
                .map_or(0, |messages| messages.iter().filter(|m| m.selected).count())
        })
    });
    let set_all = move |selected: bool| {
        draft.update(|value| {
            if let Some(messages) = value {
                for message in messages.iter_mut() {
                    message.selected = selected;
                }
            }
        });
    };
    let choose_platform = {
        let request_copy = Rc::clone(&request_copy);
        Rc::new(move |next: ShareSocialPlatform| {
            if platform.get_untracked() == next {
                return;
            }
            platform.set(next);
            social_copy.set(None);
            selected_variant.set(0);
            copy_dirty.set(false);
            error.set(None);
            let messages = draft.get_untracked().unwrap_or_default();
            edit_body.set(share_fallback_caption(&messages, next.body_limit()));
            request_copy();
        })
    };
    let export = move |_| {
        let loc = locale.get_untracked();
        let messages = draft.get_untracked().unwrap_or_default();
        let (selected, redact) = selected_share_messages(&messages, &keywords.get_untracked());
        if selected.is_empty() {
            error.set(Some(t(loc, "share.none_selected")));
            return;
        }
        busy.set(true);
        error.set(None);
        let stamp = share_stamp();
        let footer = t(loc, "share.image_footer");
        let html_format = format.get_untracked() == ShareExportFormat::Html;
        // Build the export payload up front (pure CPU); the spawned task only
        // awaits the optional canvas render and the native save call.
        let (png_payload, html_args) = if html_format {
            let rows: Vec<ShareHtmlRow> = selected
                .iter()
                .map(|message| ShareHtmlRow {
                    role: message.role,
                    label: t(loc, share_role_key(message.role)).to_string(),
                    text: redact_text(&message.text, &redact),
                })
                .collect();
            let html = share_html_document("wisp-science", &stamp, &footer, &rows);
            let args = to_value(&serde_json::json!({
                "html": html,
                "defaultName": format!("wisp-share-{stamp}.html"),
            }))
            .unwrap();
            (String::new(), Some(args))
        } else {
            let payload = share_png_payload(
                "wisp-science",
                &stamp,
                &footer,
                &share_png_rows(loc, &selected, &redact),
            );
            (payload, None)
        };
        spawn_local(async move {
            let result = match html_args {
                Some(args) => invoke_checked("save_share_html", args).await,
                None => {
                    async {
                        let png = render_share_png(&png_payload)
                            .await?
                            .as_string()
                            .unwrap_or_default();
                        let args = to_value(&serde_json::json!({
                            "pngBase64": png,
                            "defaultName": format!("wisp-share-{stamp}.png"),
                        }))
                        .unwrap();
                        invoke_checked("save_share_image", args).await
                    }
                    .await
                }
            };
            busy.set(false);
            match result {
                // A string result is the saved path; null means the user
                // cancelled the save dialog — keep the overlay open silently.
                Ok(value) => {
                    if let Some(path) = value.as_string().filter(|path| !path.is_empty()) {
                        show_toast(&tf(
                            locale.get_untracked(),
                            "share.saved",
                            &[("path", &path)],
                        ));
                        draft.set(None);
                    }
                }
                Err(value) => error.set(Some(localize_backend(
                    locale.get_untracked(),
                    &js_error_text(value),
                ))),
            }
        });
    };
    let regenerate = {
        let request_copy = Rc::clone(&request_copy);
        Rc::new(move |_| {
            copy_dirty.set(false);
            request_copy();
        })
    };
    let copy_caption = move |_| {
        let loc = locale.get_untracked();
        let text = edit_body.get_untracked();
        if text.trim().is_empty() {
            error.set(Some(t(loc, "share.need_copy")));
            return;
        }
        error.set(None);
        copy_text(text);
    };
    let copy_visual = move |include_text: bool| {
        let loc = locale.get_untracked();
        let text = if include_text {
            let body = edit_body.get_untracked();
            if body.trim().is_empty() {
                error.set(Some(t(loc, "share.need_copy")));
                return;
            }
            body
        } else {
            String::new()
        };
        let cached = cards.with_untracked(|list| {
            list.get(selected_card.get_untracked())
                .and_then(|card| card.png_base64.clone())
        });
        let fallback_payload = if cached.is_none() {
            let messages = draft.get_untracked().unwrap_or_default();
            let (selected, redact) =
                selected_share_messages(&messages, &keywords.get_untracked());
            if selected.is_empty() {
                error.set(Some(t(loc, "share.none_selected")));
                return;
            }
            Some(share_png_payload(
                "wisp-science",
                &share_stamp(),
                &t(loc, "share.image_footer"),
                &share_png_rows(loc, &selected, &redact),
            ))
        } else {
            None
        };
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = async {
                let png = if let Some(png) = cached {
                    png
                } else {
                    render_share_png(&fallback_payload.unwrap_or_default())
                        .await?
                        .as_string()
                        .unwrap_or_default()
                };
                copy_share_pack(&text, &png).await
            }
            .await;
            busy.set(false);
            match result {
                Ok(_) => show_toast(&t(
                    locale.get_untracked(),
                    if include_text {
                        "share.copied_pack"
                    } else {
                        "share.copied_image"
                    },
                )),
                Err(value) => error.set(Some(localize_backend(
                    locale.get_untracked(),
                    &js_error_text(value),
                ))),
            }
        });
    };

    move || {
        let choose_platform = Rc::clone(&choose_platform);
        let regenerate = Rc::clone(&regenerate);
        open.get().then(move || view! {
            <div class="overlay">
                <div class="modal share-modal share-social-modal" role="dialog" aria-modal="true"
                    aria-labelledby="share-modal-title" data-testid="share-overlay">
                    <div class="ps-head">
                        <h2 id="share-modal-title">{move || t(locale.get(), "share.title")}</h2>
                        <button type="button" class="ps-close"
                            title=move || t(locale.get(), "share.cancel")
                            aria-label=move || t(locale.get(), "share.cancel")
                            on:click=move |_| draft.set(None)>
                            {compose_icon("close")}
                        </button>
                    </div>
                    <p class="hint">{move || t(locale.get(), "share.hint_social")}</p>
                    <div class="share-social" data-testid="share-social">
                        <div class="share-format">
                            <span class="share-format-label">{t(locale.get(), "share.platform_label")}</span>
                            <div class="share-format-seg share-platform-seg" role="group"
                                aria-label=t(locale.get(), "share.platform_label")>
                                {ShareSocialPlatform::all().into_iter().map({
                                    let choose_platform = Rc::clone(&choose_platform);
                                    move |item| {
                                        let choose_platform = Rc::clone(&choose_platform);
                                        view! {
                                            <button type="button"
                                                data-testid=format!("share-platform-{}", item.as_str())
                                                class:active=move || platform.get() == item
                                                on:click=move |_| choose_platform(item)>
                                                {t(locale.get(), item.label_key())}
                                            </button>
                                        }
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                        <label class="share-copy-editor" for="share-copy-input">
                            {move || if generating.get() {
                                t(locale.get(), "share.generating")
                            } else {
                                t(locale.get(), "share.edit_label")
                            }}
                            <textarea id="share-copy-input" data-testid="share-copy-input"
                                prop:value=move || edit_body.get()
                                on:input=move |ev| {
                                    copy_dirty.set(true);
                                    edit_body.set(event_target_value(&ev));
                                }>
                            </textarea>
                            <span class="share-copy-meta">{move || {
                                let count = edit_body.get().chars().count();
                                let limit = platform.get().body_limit();
                                tf(locale.get(), "share.chars", &[
                                    ("count", &count.to_string()),
                                    ("limit", &limit.to_string()),
                                ])
                            }}</span>
                        </label>
                        {move || {
                            let regenerate = Rc::clone(&regenerate);
                            social_copy.get().map(move |copy| view! {
                            <div class="share-variants" role="group"
                                aria-label=t(locale.get(), "share.variants")>
                                {copy.variants.iter().enumerate().map(|(index, _)| {
                                    view! {
                                        <button type="button"
                                            data-testid=format!("share-variant-{index}")
                                            class:active=move || selected_variant.get() == index
                                            on:click=move |_| {
                                                selected_variant.set(index);
                                                copy_dirty.set(false);
                                                if let Some(variant) = social_copy
                                                    .get_untracked()
                                                    .as_ref()
                                                    .and_then(|current| current.variants.get(index))
                                                {
                                                    edit_body.set(share_social_pack_text(variant));
                                                }
                                            }>
                                            {tf(locale.get(), "share.variant_n",
                                                &[("n", &format!("{}", index + 1))])}
                                        </button>
                                    }
                                }).collect_view()}
                                <button type="button" class="share-generate" data-testid="share-generate"
                                    disabled=move || generating.get() || busy.get() || selected_count.get() == 0
                                    on:click={
                                        let regenerate = Rc::clone(&regenerate);
                                        move |_| regenerate(())
                                    }>
                                    {compose_icon("sparkles")}
                                    {move || t(locale.get(), "share.regenerate")}
                                </button>
                            </div>
                            })
                        }}
                        <div class="share-cards" data-testid="share-cards">
                            <span class="share-format-label">{move || t(locale.get(), "share.cards")}</span>
                            <div class="share-card-row">
                                {move || cards.get().into_iter().enumerate().map(|(index, card)| {
                                    let title = if card.title.is_empty() {
                                        tf(locale.get(), "share.card_n", &[("n", &format!("{}", index + 1))])
                                    } else {
                                        card.title.clone()
                                    };
                                    let src = card.png_base64.as_ref().map(|png| {
                                        format!("data:image/png;base64,{png}")
                                    });
                                    view! {
                                        <button type="button" class="share-card"
                                            data-testid=format!("share-card-{index}")
                                            class:active=move || selected_card.get() == index
                                            on:click=move |_| selected_card.set(index)>
                                            {match src {
                                                Some(src) => view! { <img src=src alt=title.clone() /> }.into_view(),
                                                None => view! {
                                                    <span class="share-card-placeholder">
                                                        {t(locale.get(), "share.card_rendering")}
                                                    </span>
                                                }.into_view(),
                                            }}
                                            <span class="share-card-label">{title}</span>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                        <div class="share-social-actions">
                            <button type="button" class="primary" data-testid="share-copy-pack"
                                disabled=move || busy.get() || edit_body.get().trim().is_empty()
                                on:click=move |_| copy_visual(true)>
                                {compose_icon("clipboard")}
                                {t(locale.get(), "share.copy_pack")}
                            </button>
                            <button type="button" data-testid="share-copy-caption"
                                disabled=move || busy.get() || edit_body.get().trim().is_empty()
                                on:click=copy_caption>
                                {compose_icon("copy")}
                                {t(locale.get(), "share.copy_caption")}
                            </button>
                            <button type="button" data-testid="share-copy-image"
                                disabled=move || busy.get()
                                on:click=move |_| copy_visual(false)>
                                {compose_icon("image")}
                                {t(locale.get(), "share.copy_image")}
                            </button>
                        </div>
                        <p class="hint share-social-footer">{t(locale.get(), "share.social_footer")}</p>
                    </div>
                    {move || error.get().map(|message| view! {
                        <div class="settings-status fail">{message}</div>
                    })}
                    <details class="share-advanced" data-testid="share-advanced">
                        <summary>{move || t(locale.get(), "share.advanced")}</summary>
                        <p class="hint">{move || t(locale.get(), "share.hint")}</p>
                        <div class="share-select-row">
                            <button type="button" class="linklike"
                                on:click=move |_| set_all(true)>{move || t(locale.get(), "share.select_all")}</button>
                            <button type="button" class="linklike"
                                on:click=move |_| set_all(false)>{move || t(locale.get(), "share.select_none")}</button>
                            <span class="share-count">{move || {
                                let total = draft.with(|value| value.as_ref().map_or(0, Vec::len));
                                tf(locale.get(), "share.selected",
                                    &[("count", &format!("{}/{}", selected_count.get(), total))])
                            }}</span>
                        </div>
                        <div class="share-list share-list-compact">
                            {move || {
                                let loc = locale.get();
                                let redact = parse_redact_keywords(&keywords.get());
                                draft.get().unwrap_or_default().into_iter().enumerate().map(|(index, message)| {
                                    let row_class = format!("share-row share-{}", message.role.tag());
                                    let preview = redact_text(&message.text, &redact);
                                    view! {
                                        <label class=row_class>
                                            <input type="checkbox" prop:checked=message.selected
                                                on:change=move |_| draft.update(|value| {
                                                    if let Some(messages) = value {
                                                        if let Some(row) = messages.get_mut(index) {
                                                            row.selected = !row.selected;
                                                        }
                                                    }
                                                }) />
                                            <span class="share-row-body">
                                                <span class="share-role">{t(loc, share_role_key(message.role))}</span>
                                                <span class="share-text">{preview}</span>
                                            </span>
                                        </label>
                                    }
                                }).collect_view()
                            }}
                        </div>
                        <label class="share-redact" for="share-redact-input">
                            {move || t(locale.get(), "share.redact_label")}
                            <input id="share-redact-input" autocomplete="off"
                                placeholder=move || t(locale.get(), "share.redact_ph")
                                prop:value=move || keywords.get()
                                on:input=move |ev| keywords.set(event_target_value(&ev)) />
                        </label>
                        <div class="share-format">
                            <span class="share-format-label">{move || t(locale.get(), "share.format_label")}</span>
                            <div class="share-format-seg" role="group"
                                aria-label=move || t(locale.get(), "share.format_label")>
                                <button type="button" data-testid="share-format-png"
                                    class:active=move || format.get() == ShareExportFormat::Png
                                    on:click=move |_| format.set(ShareExportFormat::Png)>
                                    {move || t(locale.get(), "share.format_png")}</button>
                                <button type="button" data-testid="share-format-html"
                                    class:active=move || format.get() == ShareExportFormat::Html
                                    on:click=move |_| format.set(ShareExportFormat::Html)>
                                    {move || t(locale.get(), "share.format_html")}</button>
                            </div>
                        </div>
                        <div class="row">
                            <button type="button" class="primary" data-testid="share-export"
                                disabled=move || busy.get() || selected_count.get() == 0
                                on:click=export>{move || if busy.get() {
                                    t(locale.get(), "share.exporting")
                                } else if format.get() == ShareExportFormat::Html {
                                    t(locale.get(), "share.export_html")
                                } else {
                                    t(locale.get(), "share.export")
                                }}</button>
                        </div>
                    </details>
                    <div class="row">
                        <button type="button" disabled=move || busy.get()
                            on:click=move |_| draft.set(None)>{move || t(locale.get(), "share.cancel")}</button>
                    </div>
                </div>
            </div>
        })
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

fn run_review_subtitle_label(title: Option<&str>, run_id: &str) -> (String, bool) {
    if let Some(title) = title.map(str::trim).filter(|title| !title.is_empty()) {
        (title.to_string(), false)
    } else {
        (run_id.split('-').next().unwrap_or(run_id).to_string(), true)
    }
}

fn run_review_icon(is_dir: bool, name: &str) -> &'static str {
    if is_dir {
        "folder"
    } else if file_kind(name) == Some("image") {
        "image"
    } else {
        "doc"
    }
}

fn toggle_run_review_selection(
    selection: RwSignal<std::collections::HashMap<String, String>>,
    path: String,
    kind: String,
) {
    selection.update(|selected| {
        if selected.remove(&path).is_none() {
            selected.insert(path, kind);
        }
    });
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

    let toggle_all = move |_| {
        let rows = entries.get_untracked();
        selection.update(|selected| {
            let all_on =
                !rows.is_empty() && rows.iter().all(|entry| selected.contains_key(&entry.path));
            if all_on {
                for entry in &rows {
                    selected.remove(&entry.path);
                }
            } else {
                for entry in rows {
                    selected.insert(entry.path, entry.kind);
                }
            }
        });
    };

    move || {
        modal.get().map(|run_id| {
            let (subtitle, subtitle_is_id) = runs.with(|runs| {
                run_review_subtitle_label(
                    runs.iter()
                        .find(|run| run.id == run_id)
                        .map(|run| run.title.as_str()),
                    &run_id,
                )
            });
            let subtitle_title = run_id.clone();
            view! {
                <div class="overlay">
                    <div class="modal run-review-modal" role="dialog" aria-modal="true"
                        aria-labelledby="run-review-title" data-testid="run-review-modal">
                        <div class="ps-head">
                            <div class="context-modal-title">
                                <h2 id="run-review-title">{move || t(locale.get(), "run_review.title")}</h2>
                                <p class="run-review-subtitle" class:is-id=subtitle_is_id
                                    data-testid="run-review-subtitle" title=subtitle_title>{subtitle}</p>
                            </div>
                            <button type="button" class="ps-close"
                                title=move || t(locale.get(), "settings.cancel")
                                aria-label=move || t(locale.get(), "settings.cancel")
                                on:click=move |_| close()>{compose_icon("close")}</button>
                        </div>
                        <p class="run-review-hint">{move || t(locale.get(), "run_review.hint")}</p>
                        <div class="run-review-toolbar">
                            {move || {
                                let current = path.get();
                                (!current.is_empty()).then(|| {
                                    let display = format!("/{current}");
                                    view! {
                                        <div class="run-review-pathbar">
                                            <button type="button" class="run-review-up"
                                                disabled=move || busy.get()
                                                on:click=move |_| {
                                                    path.update(|value| {
                                                        *value = value.rsplit_once('/')
                                                            .map(|(parent, _)| parent.to_string())
                                                            .unwrap_or_default();
                                                    });
                                                    fetch(false);
                                                }>
                                                {compose_icon("arrow-left")}
                                                {move || t(locale.get(), "run_review.up")}
                                            </button>
                                            <code class="run-review-path" title=display.clone()>{display}</code>
                                        </div>
                                    }
                                })
                            }}
                            <div class="run-review-filter-wrap">
                                {compose_icon("search")}
                                <input type="search" class="run-review-filter" autocomplete="off" spellcheck="false"
                                    aria-label=move || t(locale.get(), "run_review.filter")
                                    placeholder=move || t(locale.get(), "run_review.filter")
                                    prop:value=move || filter.get()
                                    on:input=move |event| {
                                        filter.set(event_target_value(&event));
                                        fetch(false);
                                    } />
                            </div>
                        </div>
                        {move || error.get().map(|message| view! {
                            <div class="settings-status fail">{message}</div>
                        })}
                        {move || status.get().map(|message| view! {
                            <div class="settings-status ok" data-testid="run-review-status">{message}</div>
                        })}
                        <div class="run-review-list" class:is-busy=move || busy.get()>
                            <div class="run-review-cols">
                                <label class="run-review-select"
                                    title=move || t(locale.get(), "run_review.select_all")>
                                    <input type="checkbox" data-testid="run-review-select-all"
                                        aria-label=move || t(locale.get(), "run_review.select_all")
                                        prop:checked=move || {
                                            let rows = entries.get();
                                            !rows.is_empty() && rows.iter().all(|entry| {
                                                selection.with(|selected| selected.contains_key(&entry.path))
                                            })
                                        }
                                        prop:indeterminate=move || {
                                            let rows = entries.get();
                                            let selected = selection.get();
                                            let n = rows.iter().filter(|entry| selected.contains_key(&entry.path)).count();
                                            n > 0 && n < rows.len()
                                        }
                                        on:change=toggle_all />
                                </label>
                                <span>{move || t(locale.get(), "run_review.col_name")}</span>
                                <span class="run-review-meta">{move || t(locale.get(), "run_review.col_size")}</span>
                            </div>
                            {move || {
                                let loc = locale.get();
                                let rows = entries.get();
                                if rows.is_empty() {
                                    view! { <div class="control-empty">{t(loc, "run_review.empty")}</div> }.into_view()
                                } else {
                                    rows.into_iter().map(|entry| {
                                        let is_dir = entry.kind == "dir";
                                        let toggle_path = entry.path.clone();
                                        let toggle_kind = entry.kind.clone();
                                        let row_path = entry.path.clone();
                                        let row_kind = entry.kind.clone();
                                        let enter_path = entry.path.clone();
                                        let checked_path = entry.path.clone();
                                        let selected_path = entry.path.clone();
                                        let name = entry.path.rsplit('/').next().unwrap_or(&entry.path).to_string();
                                        let icon = run_review_icon(is_dir, &name);
                                        let count = entry.file_count.unwrap_or(0);
                                        let meta = if is_dir {
                                            let files = if count == 1 {
                                                t(loc, "run_review.dir_file")
                                            } else {
                                                tf(loc, "run_review.dir_files", &[("n", &count.to_string())])
                                            };
                                            format!("{} · {files}", format_bytes(entry.size_bytes))
                                        } else {
                                            format_bytes(entry.size_bytes)
                                        };
                                        view! {
                                            <div class="run-review-row" data-testid="run-review-row"
                                                class:selected=move || selection.with(|selected| selected.contains_key(&selected_path))
                                                on:click=move |_| toggle_run_review_selection(
                                                    selection, row_path.clone(), row_kind.clone(),
                                                )>
                                                <label class="run-review-select"
                                                    on:click=move |event| event.stop_propagation()>
                                                    <input type="checkbox"
                                                        prop:checked=move || selection.with(|selected| selected.contains_key(&checked_path))
                                                        on:change=move |_| toggle_run_review_selection(
                                                            selection, toggle_path.clone(), toggle_kind.clone(),
                                                        ) />
                                                </label>
                                                {if is_dir {
                                                    view! {
                                                        <button type="button" class="run-review-name run-review-dir"
                                                            on:click=move |event| {
                                                                event.stop_propagation();
                                                                path.set(enter_path.clone());
                                                                fetch(false);
                                                            }>
                                                            <span class="run-review-icon">{compose_icon(icon)}</span>
                                                            <span class="run-review-filename" data-testid="run-review-name">{name}</span>
                                                        </button>
                                                    }.into_view()
                                                } else {
                                                    view! {
                                                        <span class="run-review-name">
                                                            <span class="run-review-icon">{compose_icon(icon)}</span>
                                                            <span class="run-review-filename" data-testid="run-review-name">{name}</span>
                                                        </span>
                                                    }.into_view()
                                                }}
                                                <span class="run-review-meta" data-testid="run-review-size">{meta}</span>
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
                        <p class="run-review-warning">{move || t(locale.get(), "run_review.delete_warning")}</p>
                        <div class="row run-review-actions">
                            <button type="button" class="run-review-cleanup"
                                disabled=move || busy.get()
                                on:click=cleanup_all>{move || t(locale.get(), "run_review.cleanup_all")}</button>
                            {move || {
                                let n = selection.with(|selected| selected.len());
                                (n > 0).then(|| view! {
                                    <span class="run-review-count" data-testid="run-review-count">
                                        {tf(locale.get(), "run_review.selected_n", &[("n", &n.to_string())])}
                                    </span>
                                })
                            }}
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

#[cfg(test)]
mod run_review_label_tests {
    use super::{run_review_icon, run_review_subtitle_label};

    #[test]
    fn prefers_the_run_title_over_the_id() {
        let (label, is_id) = run_review_subtitle_label(Some("Kinase screen QC"), "run-kinase-001");
        assert_eq!(label, "Kinase screen QC");
        assert!(!is_id);
    }

    #[test]
    fn falls_back_to_the_first_id_segment() {
        let (label, is_id) =
            run_review_subtitle_label(Some("   "), "55f3c6ca-fefb-4015-a083-d60d622a830e");
        assert_eq!(label, "55f3c6ca");
        assert!(is_id);
    }

    #[test]
    fn picks_icons_by_entry_kind() {
        assert_eq!(run_review_icon(true, "results"), "folder");
        assert_eq!(run_review_icon(false, "summary.png"), "image");
        assert_eq!(run_review_icon(false, "summary.tsv"), "doc");
    }
}
