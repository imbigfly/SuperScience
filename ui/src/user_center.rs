//! TCTOKEN user center overlay: login + account / tasks / topup / API keys.

use crate::app_support::{compose_icon, js_error_text, show_toast};
use crate::bindings::{invoke, invoke_checked, open_external_url};
use crate::i18n::{localize_backend, t, tf, Locale};
use crate::text::{event_target_checked, event_target_value};
use crate::window_capture_escape;
use leptos::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::JsValue;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TctokenSession {
    pub logged_in: bool,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub group: Option<String>,
}

impl TctokenSession {
    pub(crate) fn display_label(&self) -> Option<String> {
        if !self.logged_in {
            return None;
        }
        self.display_name
            .clone()
            .or_else(|| self.username.clone())
            .filter(|name| !name.trim().is_empty())
    }

    pub(crate) fn avatar_initial(&self) -> String {
        self.display_label()
            .and_then(|name| name.chars().next())
            .map(|ch| ch.to_uppercase().to_string())
            .unwrap_or_else(|| "?".into())
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct TctokenLoginResult {
    require_2fa: bool,
    session: TctokenSession,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct TctokenRememberedLogin {
    remember: bool,
    username: String,
    password: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct TctokenAccount {
    user_id: i64,
    username: String,
    display_name: String,
    group: String,
    #[allow(dead_code)]
    quota: i64,
    #[allow(dead_code)]
    used_quota: i64,
    request_count: i64,
    remaining_display: String,
    used_display: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UserCenterTab {
    Account,
    Tasks,
    Topup,
    Keys,
}

fn is_tctoken_session_expired(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("session expired")
        || lower.contains("unauthorized")
        || lower.contains("invalid access token")
        || lower.contains("not logged in.")
}

pub(crate) fn refresh_tctoken_session(session: RwSignal<TctokenSession>) {
    spawn_local(async move {
        let value = invoke("tctoken_session", JsValue::UNDEFINED).await;
        if let Ok(s) = from_value::<TctokenSession>(value) {
            session.set(s);
        }
    });
}

#[component]
pub(crate) fn UserCenterOverlay(
    locale: RwSignal<Locale>,
    show: RwSignal<bool>,
    session: RwSignal<TctokenSession>,
) -> impl IntoView {
    let tab = create_rw_signal(UserCenterTab::Account);
    let username = create_rw_signal(String::new());
    let password = create_rw_signal(String::new());
    let remember_password = create_rw_signal(false);
    let password_visible = create_rw_signal(false);
    let totp = create_rw_signal(String::new());
    let require_2fa = create_rw_signal(false);
    let busy = create_rw_signal(false);
    let status = create_rw_signal::<Option<(bool, String)>>(None);

    let apply_remembered_login = move || {
        spawn_local(async move {
            let Ok(value) =
                invoke_checked("tctoken_get_remembered_login", JsValue::UNDEFINED).await
            else {
                return;
            };
            let Ok(saved) = from_value::<TctokenRememberedLogin>(value) else {
                return;
            };
            if saved.remember {
                username.set(saved.username);
                password.set(saved.password);
                remember_password.set(true);
            }
        });
    };

    let persist_remembered_login = move |remember: bool, user: String, pass: String| {
        spawn_local(async move {
            if remember {
                let args = to_value(&serde_json::json!({
                    "username": user,
                    "password": pass,
                }))
                .unwrap();
                let _ = invoke_checked("tctoken_set_remembered_login", args).await;
            } else {
                let _ = invoke_checked("tctoken_clear_remembered_login", JsValue::UNDEFINED).await;
            }
        });
    };

    let account = create_rw_signal::<Option<TctokenAccount>>(None);
    let topup_info = create_rw_signal::<Option<Value>>(None);
    let pay_amount = create_rw_signal(String::from("50"));
    let pay_method = create_rw_signal(String::from("alipay"));
    let redeem_code = create_rw_signal(String::new());
    let provider_url = create_rw_signal(String::from("https://www.tctoken.cn"));

    let task_start = create_rw_signal(String::new());
    let task_end = create_rw_signal(String::new());
    let task_model = create_rw_signal(String::new());
    let task_group = create_rw_signal(String::new());
    let task_type = create_rw_signal(String::from("0"));
    let task_rows = create_rw_signal::<Vec<Value>>(Vec::new());
    let task_stat = create_rw_signal::<Option<Value>>(None);
    let task_total = create_rw_signal(0i64);

    let orders = create_rw_signal::<Vec<Value>>(Vec::new());
    let tokens = create_rw_signal::<Vec<Value>>(Vec::new());
    let drawing_token_id = create_rw_signal::<Option<i64>>(None);

    create_effect(move |_| {
        if !show.get() {
            return;
        }
        require_2fa.set(false);
        totp.set(String::new());
        status.set(None);
        password_visible.set(false);
        tab.set(UserCenterTab::Account);
        spawn_local(async move {
            let v = invoke("tctoken_provider_url", JsValue::UNDEFINED).await;
            if let Ok(url) = from_value::<String>(v) {
                provider_url.set(url);
            }
        });
        // A leftover local token still looks logged-in. Validate via the
        // account fetch instead of refreshing the stale profile first.
        if session.get_untracked().logged_in {
            return;
        }
        refresh_tctoken_session(session);
        apply_remembered_login();
    });

    let switch_to_login = move || {
        session.set(TctokenSession::default());
        account.set(None);
        require_2fa.set(false);
        apply_remembered_login();
        spawn_local(async move {
            let _ = invoke_checked("tctoken_logout", JsValue::UNDEFINED).await;
        });
    };

    let fail_or_expire = move |err: wasm_bindgen::JsValue| {
        let raw = js_error_text(err);
        let msg = localize_backend(locale.get_untracked(), &raw);
        if is_tctoken_session_expired(&raw) || is_tctoken_session_expired(&msg) {
            switch_to_login();
            status.set(Some((
                false,
                t(locale.get_untracked(), "user_center.session_expired").into(),
            )));
            return;
        }
        status.set(Some((false, msg)));
    };

    let load_account = move || {
        spawn_local(async move {
            busy.set(true);
            match invoke_checked("tctoken_account", JsValue::UNDEFINED).await {
                Ok(value) => match from_value::<TctokenAccount>(value) {
                    Ok(acc) => {
                        let next = TctokenSession {
                            logged_in: true,
                            user_id: Some(acc.user_id),
                            username: Some(acc.username.clone()),
                            display_name: Some(acc.display_name.clone()),
                            group: Some(acc.group.clone()),
                        };
                        if session.get_untracked() != next {
                            session.set(next);
                        }
                        account.set(Some(acc));
                    }
                    Err(err) => status.set(Some((false, err.to_string()))),
                },
                Err(err) => fail_or_expire(err),
            }
            if let Ok(value) = invoke_checked("tctoken_topup_info", JsValue::UNDEFINED).await {
                if let Ok(info) = from_value::<Value>(value) {
                    topup_info.set(Some(info.clone()));
                    if let Some((code, _)) = pay_method_options(Some(&info)).into_iter().next() {
                        pay_method.set(code);
                    }
                    if let Some(options) = info
                        .get("amount_options")
                        .and_then(|v| v.as_array())
                        .cloned()
                    {
                        if let Some(first) = options
                            .iter()
                            .filter_map(|v| v.as_i64().or_else(|| v.as_f64().map(|n| n as i64)))
                            .find(|n| *n > 0)
                        {
                            if pay_amount.get_untracked().parse::<i64>().unwrap_or(0) <= 0 {
                                pay_amount.set(first.to_string());
                            }
                        }
                    }
                }
            }
            busy.set(false);
        });
    };

    let load_tasks = move || {
        spawn_local(async move {
            busy.set(true);
            let start_ts = parse_local_date_start(&task_start.get_untracked());
            let end_ts = parse_local_date_end(&task_end.get_untracked());
            let log_type = task_type
                .get_untracked()
                .parse::<i64>()
                .ok()
                .filter(|v| *v > 0);
            let args = to_value(&serde_json::json!({
                "p": 1,
                "pageSize": 50,
                "logType": log_type,
                "startTimestamp": start_ts,
                "endTimestamp": end_ts,
                "modelName": empty_to_none(&task_model.get_untracked()),
                "group": empty_to_none(&task_group.get_untracked()),
            }))
            .unwrap();
            match invoke_checked("tctoken_logs", args).await {
                Ok(value) => {
                    if let Ok(data) = from_value::<Value>(value) {
                        let items = data
                            .get("items")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        task_total.set(data.get("total").and_then(|v| v.as_i64()).unwrap_or(0));
                        task_rows.set(items);
                    }
                }
                Err(err) => fail_or_expire(err),
            }
            let stat_args = to_value(&serde_json::json!({
                "logType": log_type,
                "startTimestamp": start_ts,
                "endTimestamp": end_ts,
                "modelName": empty_to_none(&task_model.get_untracked()),
                "group": empty_to_none(&task_group.get_untracked()),
            }))
            .unwrap();
            if let Ok(value) = invoke_checked("tctoken_logs_stat", stat_args).await {
                if let Ok(stat) = from_value::<Value>(value) {
                    task_stat.set(Some(stat));
                }
            }
            busy.set(false);
        });
    };

    let load_orders = move || {
        spawn_local(async move {
            busy.set(true);
            let args = to_value(&serde_json::json!({ "p": 1, "pageSize": 50 })).unwrap();
            match invoke_checked("tctoken_topup_orders", args).await {
                Ok(value) => {
                    if let Ok(data) = from_value::<Value>(value) {
                        let items = data
                            .get("items")
                            .or_else(|| data.as_array().map(|_| &data))
                            .and_then(|v| {
                                if let Some(arr) = v.as_array() {
                                    Some(arr.clone())
                                } else {
                                    v.get("items").and_then(|i| i.as_array()).cloned()
                                }
                            })
                            .unwrap_or_else(|| data.as_array().cloned().unwrap_or_default());
                        orders.set(items);
                    }
                }
                Err(err) => fail_or_expire(err),
            }
            busy.set(false);
        });
    };

    let load_tokens = move || {
        spawn_local(async move {
            busy.set(true);
            let args = to_value(&serde_json::json!({ "p": 1, "pageSize": 50 })).unwrap();
            match invoke_checked("tctoken_tokens", args).await {
                Ok(value) => {
                    if let Ok(data) = from_value::<Value>(value) {
                        let items = data
                            .get("items")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let ids: Vec<i64> = items
                            .iter()
                            .filter_map(|row| row.get("id").and_then(|v| v.as_i64()))
                            .filter(|id| *id > 0)
                            .collect();
                        tokens.set(items);
                        let saved =
                            invoke_checked("tctoken_get_default_token_id", JsValue::UNDEFINED)
                                .await
                                .ok()
                                .and_then(|v| from_value::<Option<i64>>(v).ok())
                                .flatten();
                        let target = saved
                            .filter(|id| ids.contains(id))
                            .or_else(|| ids.first().copied());
                        if let Some(id) = target {
                            drawing_token_id.set(Some(id));
                            // First key becomes the default when none was saved yet.
                            if saved != Some(id) {
                                let args = to_value(&serde_json::json!({ "id": id })).unwrap();
                                let _ = invoke_checked("tctoken_set_default_token", args).await;
                            }
                        } else {
                            drawing_token_id.set(None);
                        }
                    }
                }
                Err(err) => fail_or_expire(err),
            }
            busy.set(false);
        });
    };

    // Track only show/tab. Reading `session` here would re-fire after load_account
    // writes the profile back into the session signal (infinite fetch loop).
    create_effect(move |_| {
        if !show.get() {
            return;
        }
        let current = tab.get();
        if !session.get_untracked().logged_in {
            return;
        }
        match current {
            UserCenterTab::Account => load_account(),
            UserCenterTab::Tasks => load_tasks(),
            UserCenterTab::Topup => load_orders(),
            UserCenterTab::Keys => load_tokens(),
        }
    });

    let do_login = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        status.set(None);
        let user = username.get_untracked();
        let pass = password.get_untracked();
        let remember = remember_password.get_untracked();
        spawn_local(async move {
            let args = to_value(&serde_json::json!({
                "username": user,
                "password": pass,
                "refreshToken": false,
            }))
            .unwrap();
            match invoke_checked("tctoken_login", args).await {
                Ok(value) => match from_value::<TctokenLoginResult>(value) {
                    Ok(result) => {
                        if result.require_2fa {
                            require_2fa.set(true);
                            status.set(Some((
                                true,
                                t(locale.get_untracked(), "user_center.need_2fa").into(),
                            )));
                        } else {
                            require_2fa.set(false);
                            persist_remembered_login(remember, user, pass);
                            if !remember {
                                password.set(String::new());
                            }
                            password_visible.set(false);
                            session.set(result.session);
                            tab.set(UserCenterTab::Account);
                            load_account();
                            show_toast(&t(locale.get_untracked(), "user_center.login_ok"));
                        }
                    }
                    Err(err) => status.set(Some((false, err.to_string()))),
                },
                Err(err) => status.set(Some((
                    false,
                    localize_backend(locale.get_untracked(), &js_error_text(err)),
                ))),
            }
            busy.set(false);
        });
    };

    let do_2fa = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        status.set(None);
        let code = totp.get_untracked();
        let user = username.get_untracked();
        let pass = password.get_untracked();
        let remember = remember_password.get_untracked();
        spawn_local(async move {
            let args = to_value(&serde_json::json!({
                "code": code,
                "refreshToken": false,
            }))
            .unwrap();
            match invoke_checked("tctoken_login_2fa", args).await {
                Ok(value) => match from_value::<TctokenLoginResult>(value) {
                    Ok(result) => {
                        require_2fa.set(false);
                        totp.set(String::new());
                        persist_remembered_login(remember, user, pass);
                        if !remember {
                            password.set(String::new());
                        }
                        password_visible.set(false);
                        session.set(result.session);
                        tab.set(UserCenterTab::Account);
                        load_account();
                        show_toast(&t(locale.get_untracked(), "user_center.login_ok"));
                    }
                    Err(err) => status.set(Some((false, err.to_string()))),
                },
                Err(err) => status.set(Some((
                    false,
                    localize_backend(locale.get_untracked(), &js_error_text(err)),
                ))),
            }
            busy.set(false);
        });
    };

    let do_logout = move || {
        spawn_local(async move {
            let _ = invoke_checked("tctoken_logout", JsValue::UNDEFINED).await;
            session.set(TctokenSession::default());
            account.set(None);
            require_2fa.set(false);
            password_visible.set(false);
            apply_remembered_login();
            show_toast(&t(locale.get_untracked(), "user_center.logout_ok"));
        });
    };

    window_capture_escape(move || {
        if !show.get_untracked() {
            return false;
        }
        show.set(false);
        true
    });

    move || {
        show.get().then(|| view! {
            <div class="overlay user-center-overlay" data-testid="user-center-dialog">
                <div class="modal user-center-modal" role="dialog" aria-modal="true"
                    aria-labelledby="user-center-title">
                    <div class="ps-head">
                        <div class="user-center-title-block">
                            <h2 id="user-center-title">{move || t(locale.get(), "user_center.title")}</h2>
                            <span class="user-center-subtitle">{move || {
                                session.get().display_name
                                    .or(session.get().username)
                                    .unwrap_or_else(|| t(locale.get(), "user_center.guest").into())
                            }}</span>
                        </div>
                        <button type="button" class="ps-close"
                            title=move || t(locale.get(), "user_center.close")
                            aria-label=move || t(locale.get(), "user_center.close")
                            on:click=move |_| show.set(false)>
                            {compose_icon("close")}
                        </button>
                    </div>

                    {move || (!session.get().logged_in).then(|| view! {
                        <div class="user-center-login" data-testid="user-center-login">
                            <p class="user-center-hint">
                                {move || t(locale.get(), "user_center.login_hint_before")}
                                <button type="button" class="linklike user-center-hint-link"
                                    data-testid="user-center-tctoken-link"
                                    title="https://www.tctoken.cn"
                                    on:click=move |_| {
                                        let url = provider_url.get_untracked();
                                        let url = url.trim();
                                        open_external_url(if url.is_empty() {
                                            "https://www.tctoken.cn".into()
                                        } else {
                                            url.to_string()
                                        });
                                    }>
                                    {move || t(locale.get(), "user_center.login_hint_link")}
                                </button>
                                {move || t(locale.get(), "user_center.login_hint_after")}
                            </p>
                            <label class="user-center-field">
                                <span>{move || t(locale.get(), "user_center.username")}</span>
                                <input type="text" prop:value=move || username.get()
                                    on:input=move |ev| username.set(event_target_value(&ev))
                                    data-testid="user-center-username" />
                            </label>
                            <label class="user-center-field">
                                <span>{move || t(locale.get(), "user_center.password")}</span>
                                <div class="user-center-password-wrap">
                                    <input
                                        type=move || if password_visible.get() { "text" } else { "password" }
                                        prop:value=move || password.get()
                                        autocomplete="current-password"
                                        on:input=move |ev| password.set(event_target_value(&ev))
                                        data-testid="user-center-password" />
                                    <button type="button" class="user-center-password-toggle"
                                        data-testid="user-center-password-toggle"
                                        title=move || t(
                                            locale.get(),
                                            if password_visible.get() {
                                                "user_center.hide_password"
                                            } else {
                                                "user_center.show_password"
                                            },
                                        )
                                        aria-label=move || t(
                                            locale.get(),
                                            if password_visible.get() {
                                                "user_center.hide_password"
                                            } else {
                                                "user_center.show_password"
                                            },
                                        )
                                        on:click=move |_| password_visible.update(|v| *v = !*v)>
                                        {move || compose_icon(if password_visible.get() { "eye-off" } else { "eye" })}
                                    </button>
                                </div>
                            </label>
                            {move || require_2fa.get().then(|| view! {
                                <label class="user-center-field">
                                    <span>{move || t(locale.get(), "user_center.totp")}</span>
                                    <input type="text" prop:value=move || totp.get()
                                        on:input=move |ev| totp.set(event_target_value(&ev))
                                        data-testid="user-center-totp" />
                                </label>
                            })}
                            <div class="user-center-actions">
                                <label class="user-center-remember" data-testid="user-center-remember">
                                    <input type="checkbox"
                                        prop:checked=move || remember_password.get()
                                        on:change=move |ev| {
                                            let checked = event_target_checked(&ev);
                                            remember_password.set(checked);
                                            if !checked {
                                                persist_remembered_login(
                                                    false,
                                                    String::new(),
                                                    String::new(),
                                                );
                                            }
                                        } />
                                    <span>{move || t(locale.get(), "user_center.remember_password")}</span>
                                </label>
                                {move || if require_2fa.get() {
                                    view! {
                                        <button type="button" class="primary" data-testid="user-center-2fa"
                                            disabled=move || busy.get()
                                            on:click=do_2fa>
                                            {move || t(locale.get(), "user_center.verify_2fa")}
                                        </button>
                                    }.into_view()
                                } else {
                                    view! {
                                        <button type="button" class="primary" data-testid="user-center-login-btn"
                                            disabled=move || busy.get()
                                            on:click=do_login>
                                            {move || t(locale.get(), "user_center.login")}
                                        </button>
                                    }.into_view()
                                }}
                            </div>
                        </div>
                    })}

                    {move || session.get().logged_in.then(|| {
                        let tabs = vec![
                            (UserCenterTab::Account, "user_center.tab.account"),
                            (UserCenterTab::Tasks, "user_center.tab.tasks"),
                            (UserCenterTab::Topup, "user_center.tab.topup"),
                            (UserCenterTab::Keys, "user_center.tab.keys"),
                        ];
                        view! {
                            <div class="user-center-signed-in">
                                <div class="user-center-tabs" role="tablist">
                                    {tabs.into_iter().map(|(id, key)| {
                                        view! {
                                            <button type="button" class="user-center-tab" role="tab"
                                                class:active=move || tab.get() == id
                                                data-testid=format!("user-center-tab-{}", key.rsplit('.').next().unwrap_or("x"))
                                                on:click=move |_| tab.set(id)>
                                                {move || t(locale.get(), key)}
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                                <div class="user-center-body">
                                    {move || match tab.get() {
                                        UserCenterTab::Account => view! {
                                            <AccountPane
                                                locale=locale
                                                account=account
                                                topup_info=topup_info
                                                pay_amount=pay_amount
                                                pay_method=pay_method
                                                redeem_code=redeem_code
                                                provider_url=provider_url
                                                busy=busy
                                                status=status
                                                on_refresh=Callback::new(move |_| load_account())
                                                on_logout=Callback::new(move |_| do_logout())
                                            />
                                        }.into_view(),
                                        UserCenterTab::Tasks => view! {
                                            <TasksPane
                                                locale=locale
                                                task_start=task_start
                                                task_end=task_end
                                                task_model=task_model
                                                task_group=task_group
                                                task_type=task_type
                                                task_rows=task_rows
                                                task_stat=task_stat
                                                task_total=task_total
                                                on_search=Callback::new(move |_| load_tasks())
                                                on_reset=Callback::new(move |_| {
                                                    task_start.set(String::new());
                                                    task_end.set(String::new());
                                                    task_model.set(String::new());
                                                    task_group.set(String::new());
                                                    task_type.set("0".into());
                                                    load_tasks();
                                                })
                                            />
                                        }.into_view(),
                                        UserCenterTab::Topup => view! {
                                            <OrdersPane
                                                locale=locale
                                                orders=orders
                                                on_refresh=Callback::new(move |_| load_orders())
                                            />
                                        }.into_view(),
                                        UserCenterTab::Keys => view! {
                                            <KeysPane
                                                locale=locale
                                                tokens=tokens
                                                drawing_token_id=drawing_token_id
                                                status=status
                                                busy=busy
                                            />
                                        }.into_view(),
                                    }}
                                </div>
                            </div>
                        }
                    })}

                    {move || status.get().map(|(ok, msg)| view! {
                        <p class="user-center-status" class:ok=ok class:fail=!ok data-testid="user-center-status">{msg}</p>
                    })}
                </div>
            </div>
        })
    }
}

#[component]
fn AccountPane(
    locale: RwSignal<Locale>,
    account: RwSignal<Option<TctokenAccount>>,
    topup_info: RwSignal<Option<Value>>,
    pay_amount: RwSignal<String>,
    pay_method: RwSignal<String>,
    redeem_code: RwSignal<String>,
    provider_url: RwSignal<String>,
    busy: RwSignal<bool>,
    status: RwSignal<Option<(bool, String)>>,
    on_refresh: Callback<()>,
    on_logout: Callback<()>,
) -> impl IntoView {
    let presets = [50, 100, 200, 500, 1000, 2000, 5000];
    let pay = move |_| {
        let amount = pay_amount.get_untracked().parse::<i64>().unwrap_or(0);
        let method = pay_method.get_untracked();
        if amount <= 0 {
            status.set(Some((
                false,
                t(locale.get_untracked(), "user_center.invalid_amount").into(),
            )));
            return;
        }
        let methods = pay_method_options(topup_info.get_untracked().as_ref());
        if methods.is_empty() {
            status.set(Some((
                false,
                t(locale.get_untracked(), "user_center.pay_unavailable").into(),
            )));
            return;
        }
        busy.set(true);
        status.set(None);
        spawn_local(async move {
            // Call Apifox Open API `/api/open/v1/topup/pay` via Tauri.
            // Do not send `channel` for 易支付 methods (alipay/wxpay).
            let mut payload = serde_json::json!({
                "amount": amount,
                "paymentMethod": method,
            });
            if matches!(method.as_str(), "alipay_official" | "wechat_official") {
                payload["channel"] = Value::String(
                    if method == "wechat_official" {
                        "native"
                    } else {
                        "pc"
                    }
                    .into(),
                );
            }
            let args = to_value(&payload).unwrap();
            match invoke_checked("tctoken_topup_pay", args).await {
                Ok(value) => {
                    if let Ok(data) = from_value::<Value>(value) {
                        if let Some(url) = extract_pay_url(&data) {
                            open_external_url(url);
                            show_toast(&t(locale.get_untracked(), "user_center.pay_opened"));
                            on_refresh.call(());
                        } else {
                            status.set(Some((
                                false,
                                t(locale.get_untracked(), "user_center.pay_no_url").into(),
                            )));
                        }
                    }
                }
                Err(err) => status.set(Some((
                    false,
                    localize_backend(locale.get_untracked(), &js_error_text(err)),
                ))),
            }
            busy.set(false);
        });
    };
    let redeem = move |_| {
        let key = redeem_code.get_untracked();
        if key.trim().is_empty() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let args = to_value(&serde_json::json!({ "key": key })).unwrap();
            match invoke_checked("tctoken_topup_redeem", args).await {
                Ok(_) => {
                    redeem_code.set(String::new());
                    show_toast(&t(locale.get_untracked(), "user_center.redeem_ok"));
                    on_refresh.call(());
                }
                Err(err) => status.set(Some((
                    false,
                    localize_backend(locale.get_untracked(), &js_error_text(err)),
                ))),
            }
            busy.set(false);
        });
    };
    view! {
        <div class="uc-account" data-testid="user-center-account">
            {move || account.get().map(|acc| view! {
                <div class="uc-account-hero">
                    <div class="uc-user-line">
                        <div class="uc-user-main">
                            <div class="uc-card-label">{move || t(locale.get(), "user_center.current_user")}</div>
                            <div class="uc-card-title">{acc.display_name.clone()}</div>
                            <div class="uc-card-meta">{format!("{} · {}", acc.username, acc.group)}</div>
                        </div>
                    </div>
                    <div class="uc-balance-row">
                        <div class="uc-balance">
                            <div class="uc-card-label">{move || t(locale.get(), "user_center.remaining")}</div>
                            <div class="uc-balance-value">{acc.remaining_display.clone()}</div>
                        </div>
                        <div class="uc-balance">
                            <div class="uc-card-label">{move || t(locale.get(), "user_center.used")}</div>
                            <div class="uc-balance-value">{acc.used_display.clone()}</div>
                        </div>
                        <button type="button" class="uc-refresh" on:click=move |_| on_refresh.call(())>
                            {move || t(locale.get(), "user_center.refresh")}
                        </button>
                    </div>
                </div>
                <section class="uc-section">
                    <h3>{move || t(locale.get(), "user_center.online_topup")}</h3>
                    <p class="user-center-hint">{move || t(locale.get(), "user_center.topup_hint")}</p>
                    <div class="uc-amount-presets">
                        {presets.into_iter().map(|n| {
                            let label = format!("¥{n}");
                            view! {
                                <button type="button" class="uc-chip"
                                    class:active=move || pay_amount.get() == n.to_string()
                                    on:click=move |_| pay_amount.set(n.to_string())>
                                    {label}
                                </button>
                            }
                        }).collect_view()}
                    </div>
                    <label class="user-center-field">
                        <span>{move || t(locale.get(), "user_center.custom_amount")}</span>
                        <input type="number" min="1" prop:value=move || pay_amount.get()
                            on:input=move |ev| pay_amount.set(event_target_value(&ev)) />
                    </label>
                    <label class="user-center-field">
                        <span>{move || t(locale.get(), "user_center.payment_method")}</span>
                        <select prop:value=move || pay_method.get()
                            on:change=move |ev| pay_method.set(event_target_value(&ev))>
                            {move || {
                                let options = pay_method_options(topup_info.get().as_ref());
                                let options = if options.is_empty() {
                                    vec![
                                        ("alipay".into(), "支付宝".into()),
                                        ("wxpay".into(), "微信".into()),
                                    ]
                                } else {
                                    options
                                };
                                options.into_iter().map(|(value, label)| {
                                    view! { <option value=value>{label}</option> }
                                }).collect_view()
                            }}
                        </select>
                    </label>
                    <button type="button" class="primary uc-pay-btn" disabled=move || busy.get()
                        data-testid="user-center-pay" on:click=pay>
                        {move || t(locale.get(), "user_center.go_pay")}
                    </button>
                </section>
                <section class="uc-section">
                    <h3>{move || t(locale.get(), "user_center.redeem")}</h3>
                    <div class="uc-redeem-row">
                        <input type="text" placeholder=move || t(locale.get(), "user_center.redeem_ph")
                            prop:value=move || redeem_code.get()
                            on:input=move |ev| redeem_code.set(event_target_value(&ev))
                            data-testid="user-center-redeem-input" />
                        <button type="button" disabled=move || busy.get()
                            data-testid="user-center-redeem" on:click=redeem>
                            {move || t(locale.get(), "user_center.redeem_btn")}
                        </button>
                    </div>
                </section>
                <p class="uc-footer-meta">
                    {move || tf(
                        locale.get(),
                        "user_center.request_meta",
                        &[
                            ("n", &acc.request_count.to_string()),
                            ("url", &provider_url.get()),
                        ],
                    )}
                </p>
                <button type="button" class="uc-logout" data-testid="user-center-logout"
                    on:click=move |_| on_logout.call(())>
                    {move || t(locale.get(), "user_center.logout")}
                </button>
            })}
        </div>
    }
}

#[component]
fn TasksPane(
    locale: RwSignal<Locale>,
    task_start: RwSignal<String>,
    task_end: RwSignal<String>,
    task_model: RwSignal<String>,
    task_group: RwSignal<String>,
    task_type: RwSignal<String>,
    task_rows: RwSignal<Vec<Value>>,
    task_stat: RwSignal<Option<Value>>,
    task_total: RwSignal<i64>,
    on_search: Callback<()>,
    on_reset: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="uc-tasks" data-testid="user-center-tasks">
            <div class="uc-filters">
                <label><span>{move || t(locale.get(), "user_center.start_time")}</span>
                    <input type="date" prop:value=move || task_start.get()
                        on:input=move |ev| task_start.set(event_target_value(&ev)) /></label>
                <label><span>{move || t(locale.get(), "user_center.end_time")}</span>
                    <input type="date" prop:value=move || task_end.get()
                        on:input=move |ev| task_end.set(event_target_value(&ev)) /></label>
                <label><span>{move || t(locale.get(), "user_center.model_name")}</span>
                    <input type="text" prop:value=move || task_model.get()
                        on:input=move |ev| task_model.set(event_target_value(&ev)) /></label>
                <label><span>{move || t(locale.get(), "user_center.group")}</span>
                    <input type="text" prop:value=move || task_group.get()
                        on:input=move |ev| task_group.set(event_target_value(&ev)) /></label>
                <label><span>{move || t(locale.get(), "user_center.log_type")}</span>
                    <select prop:value=move || task_type.get()
                        on:change=move |ev| task_type.set(event_target_value(&ev))>
                        <option value="0">{move || t(locale.get(), "user_center.type_all")}</option>
                        <option value="1">{move || t(locale.get(), "user_center.type_topup")}</option>
                        <option value="2">{move || t(locale.get(), "user_center.type_consume")}</option>
                        <option value="5">{move || t(locale.get(), "user_center.type_error")}</option>
                    </select>
                </label>
                <button type="button" on:click=move |_| on_reset.call(())>{move || t(locale.get(), "user_center.reset")}</button>
                <button type="button" class="primary" data-testid="user-center-task-search"
                    on:click=move |_| on_search.call(())>{move || t(locale.get(), "user_center.search")}</button>
            </div>
            <div class="uc-stat-row">
                {move || task_stat.get().map(|stat| {
                    let quota = stat.get("quota").and_then(|v| v.as_i64()).unwrap_or(0);
                    let rpm = stat.get("rpm").cloned().unwrap_or(Value::Null);
                    let tpm = stat.get("tpm").cloned().unwrap_or(Value::Null);
                    view! {
                        <span class="uc-chip soft">{format!("{} {}", t(locale.get(), "user_center.usage"), quota)}</span>
                        <span class="uc-chip soft">{format!("RPM {rpm}")}</span>
                        <span class="uc-chip soft">{format!("TPM {tpm}")}</span>
                    }
                })}
                <span class="uc-total">{move || tf(locale.get(), "user_center.total_n", &[("n", &task_total.get().to_string())])}</span>
            </div>
            <div class="uc-table-wrap">
                <table class="uc-table">
                    <thead>
                        <tr>
                            <th>{move || t(locale.get(), "user_center.col.time")}</th>
                            <th>{move || t(locale.get(), "user_center.col.channel")}</th>
                            <th>{move || t(locale.get(), "user_center.col.user")}</th>
                            <th>{move || t(locale.get(), "user_center.col.token")}</th>
                            <th>{move || t(locale.get(), "user_center.col.model")}</th>
                            <th>{move || t(locale.get(), "user_center.col.duration")}</th>
                            <th>{move || t(locale.get(), "user_center.col.tokens")}</th>
                            <th>{move || t(locale.get(), "user_center.col.cost")}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || task_rows.get().into_iter().map(|row| {
                            let created = row.get("created_at").and_then(|v| v.as_i64()).unwrap_or(0);
                            let channel = row.get("channel").and_then(|v| v.as_i64()).unwrap_or(0);
                            let user = row.get("username").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                            let token = row.get("token_name").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                            let model = row.get("model_name").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                            let use_time = row.get("use_time").and_then(|v| v.as_i64()).unwrap_or(0);
                            let prompt = row.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                            let completion = row.get("completion_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                            let quota = row.get("quota").and_then(|v| v.as_i64()).unwrap_or(0);
                            view! {
                                <tr>
                                    <td>{format_unix(created)}</td>
                                    <td>{format!("#{channel}")}</td>
                                    <td>{user}</td>
                                    <td>{token}</td>
                                    <td>{model}</td>
                                    <td>{format!("{use_time}s")}</td>
                                    <td>{format!("{prompt} / {completion}")}</td>
                                    <td>{format!("¥{:.4}", quota as f64 / 500_000.0)}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

#[component]
fn OrdersPane(
    locale: RwSignal<Locale>,
    orders: RwSignal<Vec<Value>>,
    on_refresh: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="uc-orders" data-testid="user-center-orders">
            <div class="uc-orders-head">
                <p class="user-center-hint">{move || t(locale.get(), "user_center.orders_hint")}</p>
                <button type="button" class="uc-refresh" on:click=move |_| on_refresh.call(())>
                    {move || t(locale.get(), "user_center.refresh")}
                </button>
            </div>
            <div class="uc-order-list">
                {move || {
                    let rows = orders.get();
                    if rows.is_empty() {
                        return view! {
                            <div class="uc-empty">{move || t(locale.get(), "user_center.orders_empty")}</div>
                        }.into_view();
                    }
                    rows.into_iter().map(|row| {
                        let amount = row.get("money")
                            .or_else(|| row.get("amount"))
                            .map(value_to_string)
                            .unwrap_or_else(|| "-".into());
                        let trade = row.get("trade_no")
                            .or_else(|| row.get("tradeNo"))
                            .map(value_to_string)
                            .unwrap_or_else(|| "-".into());
                        let method = row.get("payment_method")
                            .or_else(|| row.get("gateway"))
                            .or_else(|| row.get("provider"))
                            .map(value_to_string)
                            .unwrap_or_else(|| "-".into());
                        let created = row.get("created_at")
                            .or_else(|| row.get("create_time"))
                            .map(|v| {
                                if let Some(n) = v.as_i64() {
                                    format_unix(n)
                                } else {
                                    value_to_string(v)
                                }
                            })
                            .unwrap_or_else(|| "-".into());
                        let status = row.get("status")
                            .map(value_to_string)
                            .unwrap_or_else(|| "-".into());
                        let qty = row.get("quota")
                            .or_else(|| row.get("amount"))
                            .map(value_to_string)
                            .unwrap_or_else(|| "-".into());
                        view! {
                            <div class="uc-order-card">
                                <div class="uc-order-top">
                                    <strong>{format!("¥{amount}")}</strong>
                                    <span class="uc-order-status">{status}</span>
                                </div>
                                <div class="uc-order-id">{trade}</div>
                                <div class="uc-order-meta">{format!("{method} · {created} · {} {qty}", t(locale.get(), "user_center.qty"))}</div>
                            </div>
                        }
                    }).collect_view()
                }}
            </div>
        </div>
    }
}

#[component]
fn KeysPane(
    locale: RwSignal<Locale>,
    tokens: RwSignal<Vec<Value>>,
    drawing_token_id: RwSignal<Option<i64>>,
    status: RwSignal<Option<(bool, String)>>,
    busy: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="uc-keys" data-testid="user-center-keys">
            <p class="user-center-hint">{move || t(locale.get(), "user_center.keys_hint")}</p>
            <div class="uc-key-list">
                {move || tokens.get().into_iter().map(|row| {
                    let id = row.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                    let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("Token").to_string();
                    let key_mask = row.get("key").and_then(|v| v.as_str()).unwrap_or("sk-***").to_string();
                    let remain = row.get("remain_quota")
                        .or_else(|| row.get("quota"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    let expired = row.get("expired_time").and_then(|v| v.as_i64()).unwrap_or(-1);
                    let status_label = if row.get("status").and_then(|v| v.as_i64()).unwrap_or(1) == 1 {
                        t(locale.get(), "user_center.key_active")
                    } else {
                        t(locale.get(), "user_center.key_disabled")
                    };
                    let expire_label = if expired < 0 {
                        t(locale.get(), "user_center.never_expires").into()
                    } else {
                        format_unix(expired)
                    };
                    let is_drawing = move || drawing_token_id.get() == Some(id);
                    view! {
                        <div class="uc-key-card" class:active=is_drawing>
                            <div class="uc-key-top">
                                <strong>{name}</strong>
                                <span class="uc-key-status">{status_label}</span>
                            </div>
                            <div class="uc-key-mask">{key_mask}</div>
                            <div class="uc-key-meta">
                                {tf(
                                    locale.get(),
                                    "user_center.key_meta",
                                    &[
                                        ("quota", &format!("¥{:.2}", remain as f64 / 500_000.0)),
                                        ("exp", &expire_label),
                                    ],
                                )}
                            </div>
                            <div class="uc-key-actions">
                                <button type="button" on:click=move |_| {
                                    busy.set(true);
                                    spawn_local(async move {
                                        let args = to_value(&serde_json::json!({ "id": id })).unwrap();
                                        match invoke_checked("tctoken_token_key", args).await {
                                            Ok(value) => {
                                                if let Ok(data) = from_value::<Value>(value) {
                                                    if let Some(key) = data.get("key").and_then(|v| v.as_str()) {
                                                        copy_text(key);
                                                        show_toast(&t(locale.get_untracked(), "user_center.key_copied"));
                                                    }
                                                }
                                            }
                                            Err(err) => status.set(Some((
                                                false,
                                                localize_backend(locale.get_untracked(), &js_error_text(err)),
                                            ))),
                                        }
                                        busy.set(false);
                                    });
                                }>
                                    {move || t(locale.get(), "user_center.copy_key")}
                                </button>
                                <button type="button" class:primary=move || !is_drawing()
                                    disabled=move || busy.get() || is_drawing()
                                    on:click=move |_| {
                                        busy.set(true);
                                        spawn_local(async move {
                                            let args = to_value(&serde_json::json!({ "id": id })).unwrap();
                                            match invoke_checked("tctoken_set_default_token", args).await {
                                                Ok(_) => {
                                                    drawing_token_id.set(Some(id));
                                                    show_toast(&t(locale.get_untracked(), "user_center.drawing_key_set"));
                                                }
                                                Err(err) => status.set(Some((
                                                    false,
                                                    localize_backend(locale.get_untracked(), &js_error_text(err)),
                                                ))),
                                            }
                                            busy.set(false);
                                        });
                                    }>
                                    {move || if is_drawing() {
                                        t(locale.get(), "user_center.drawing_key_current")
                                    } else {
                                        t(locale.get(), "user_center.set_drawing_key")
                                    }}
                                </button>
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

fn empty_to_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn pay_method_options(info: Option<&Value>) -> Vec<(String, String)> {
    let Some(info) = info else {
        return Vec::new();
    };
    let Some(arr) = info
        .get("pay_methods")
        .or_else(|| info.get("payment_methods"))
        .and_then(|v| v.as_array())
    else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| {
            if let Some(code) = item.as_str() {
                let code = code.trim();
                if code.is_empty() {
                    return None;
                }
                return Some((code.to_string(), pay_method_label(code)));
            }
            let code = item.get("type").and_then(|v| v.as_str())?.trim();
            if code.is_empty() {
                return None;
            }
            let label = item
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| pay_method_label(code));
            Some((code.to_string(), label))
        })
        .collect()
}

fn pay_method_label(code: &str) -> String {
    match code {
        "alipay" | "alipay_official" => "支付宝".into(),
        "wxpay" | "wechat_official" | "wx" => "微信".into(),
        other => other.to_string(),
    }
}

fn extract_pay_url(data: &Value) -> Option<String> {
    for key in ["pay_url", "url"] {
        if let Some(url) = data
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return Some(url.to_string());
        }
    }
    let params = data.get("params")?;
    for key in ["payurl", "pay_url", "url", "qrcode", "code_url", "mweb_url"] {
        if let Some(url) = params
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return Some(url.to_string());
        }
    }
    None
}

fn parse_local_date_start(value: &str) -> Option<i64> {
    parse_date_seconds(value, false)
}

fn parse_local_date_end(value: &str) -> Option<i64> {
    parse_date_seconds(value, true)
}

fn parse_date_seconds(value: &str, end_of_day: bool) -> Option<i64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let suffix = if end_of_day { "T23:59:59" } else { "T00:00:00" };
    let millis = js_sys::Date::parse(&format!("{value}{suffix}"));
    if millis.is_nan() {
        None
    } else {
        Some((millis / 1000.0) as i64)
    }
}

fn format_unix(ts: i64) -> String {
    if ts <= 0 {
        return "-".into();
    }
    let date = js_sys::Date::new(&JsValue::from_f64((ts as f64) * 1000.0));
    let y = date.get_full_year();
    let mo = date.get_month() + 1;
    let d = date.get_date();
    let h = date.get_hours();
    let mi = date.get_minutes();
    let s = date.get_seconds();
    format!("{y:04}/{mo:02}/{d:02} {h:02}:{mi:02}:{s:02}")
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

fn copy_text(text: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.navigator().clipboard().write_text(text);
    }
}

#[cfg(test)]
mod tests {
    use super::{is_tctoken_session_expired, TctokenSession};

    #[test]
    fn treats_invalid_access_token_as_expired_session() {
        assert!(is_tctoken_session_expired(
            "Unauthorized, invalid access token"
        ));
        assert!(is_tctoken_session_expired(
            "Session expired. Please log in again."
        ));
        assert!(!is_tctoken_session_expired("Amount must be positive."));
    }

    #[test]
    fn display_label_prefers_display_name_when_signed_in() {
        let session = TctokenSession {
            logged_in: true,
            username: Some("demo".into()),
            display_name: Some("Demo User".into()),
            ..TctokenSession::default()
        };
        assert_eq!(session.display_label().as_deref(), Some("Demo User"));
        assert_eq!(session.avatar_initial(), "D");
    }

    #[test]
    fn display_label_is_none_when_signed_out() {
        let session = TctokenSession {
            logged_in: false,
            username: Some("demo".into()),
            display_name: Some("Demo User".into()),
            ..TctokenSession::default()
        };
        assert_eq!(session.display_label(), None);
        assert_eq!(session.avatar_initial(), "?");
    }
}
