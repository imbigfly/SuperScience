//! Fork-owned Example-project demo save / delete helpers.
//!
//! Upstream does not have these commands wired in `main.rs`. Keep the invoke
//! bodies here so a merge that rewrites the context-menu match cannot drop them.

use crate::app_support::{js_error_text, show_toast};
use crate::bindings::{invoke, invoke_checked};
use crate::dto::DemoInfo;
use crate::i18n::{localize_backend, tf, Locale};
use leptos::*;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::JsValue;

pub(crate) fn refresh_demo_list(demos: RwSignal<Vec<DemoInfo>>) {
    spawn_local(async move {
        let value = invoke("list_demos", JsValue::UNDEFINED).await;
        if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<DemoInfo>>(value) {
            demos.set(list);
        }
    });
}

pub(crate) fn save_session_as_demo(
    id: String,
    title: String,
    locale: RwSignal<Locale>,
    demos: RwSignal<Vec<DemoInfo>>,
) {
    spawn_local(async move {
        let arg = to_value(&serde_json::json!({
            "sessionId": id,
            "title": title.clone(),
        }))
        .unwrap();
        match invoke_checked("save_session_as_demo", arg).await {
            Ok(_) => {
                show_toast(&tf(
                    locale.get_untracked(),
                    "demo.save_success",
                    &[("title", &title)],
                ));
                refresh_demo_list(demos);
            }
            Err(error) => show_toast(&localize_backend(
                locale.get_untracked(),
                &js_error_text(error),
            )),
        }
    });
}

pub(crate) fn delete_user_demo(
    id: String,
    locale: RwSignal<Locale>,
    demos: RwSignal<Vec<DemoInfo>>,
) {
    spawn_local(async move {
        let arg = to_value(&serde_json::json!({ "id": id })).unwrap();
        match invoke_checked("delete_user_demo", arg).await {
            Ok(_) => refresh_demo_list(demos),
            Err(error) => show_toast(&localize_backend(
                locale.get_untracked(),
                &js_error_text(error),
            )),
        }
    });
}
