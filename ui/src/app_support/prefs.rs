use super::*;

const MODEL_SWITCH_WARNING_DISABLED_KEY: &str = "wisp-model-switch-warning-disabled";

const PRIVACY_MODE_ACTIVE_KEY: &str = "wisp-privacy-mode-active";
const PRIVACY_MODE_PROJECTS_KEY: &str = "wisp-privacy-mode-projects";

pub(crate) fn load_privacy_mode() -> (bool, HashSet<String>) {
    let storage = web_sys::window().and_then(|window| window.local_storage().ok().flatten());
    let projects = storage
        .as_ref()
        .and_then(|storage| storage.get_item(PRIVACY_MODE_PROJECTS_KEY).ok().flatten())
        .and_then(|value| serde_json::from_str::<HashSet<String>>(&value).ok())
        .unwrap_or_default();
    let active = !projects.is_empty()
        && storage
            .and_then(|storage| storage.get_item(PRIVACY_MODE_ACTIVE_KEY).ok().flatten())
            .is_some_and(|value| value == "1");
    (active, projects)
}

pub(crate) fn save_privacy_mode(active: bool, projects: &HashSet<String>) {
    let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    else {
        return;
    };
    if let Ok(value) = serde_json::to_string(projects) {
        let _ = storage.set_item(PRIVACY_MODE_PROJECTS_KEY, &value);
    }
    let _ = if active && !projects.is_empty() {
        storage.set_item(PRIVACY_MODE_ACTIVE_KEY, "1")
    } else {
        storage.remove_item(PRIVACY_MODE_ACTIVE_KEY)
    };
}

pub(crate) fn model_switch_warning_disabled() -> bool {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| {
            storage
                .get_item(MODEL_SWITCH_WARNING_DISABLED_KEY)
                .ok()
                .flatten()
        })
        .is_some_and(|value| value == "1")
}

pub(crate) fn disable_model_switch_warning() {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(MODEL_SWITCH_WARNING_DISABLED_KEY, "1");
    }
}

const SELECTION_POPUP_DISABLED_KEY: &str = "selectionPopupDisabled";

const SEND_WITH_MODIFIER_KEY: &str = "wisp-send-with-modifier";

pub(crate) fn load_selection_popup_enabled() -> bool {
    !web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(SELECTION_POPUP_DISABLED_KEY).ok().flatten())
        .is_some_and(|v| v == "1")
}

pub(crate) fn save_selection_popup_enabled(enabled: bool) {
    if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = if enabled {
            s.remove_item(SELECTION_POPUP_DISABLED_KEY)
        } else {
            s.set_item(SELECTION_POPUP_DISABLED_KEY, "1")
        };
    }
}

pub(crate) fn load_send_with_modifier() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(SEND_WITH_MODIFIER_KEY).ok().flatten())
        .is_some_and(|v| v == "1")
}

pub(crate) fn save_send_with_modifier(enabled: bool) {
    if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = if enabled {
            s.set_item(SEND_WITH_MODIFIER_KEY, "1")
        } else {
            s.remove_item(SEND_WITH_MODIFIER_KEY)
        };
    }
}

pub(crate) fn load_theme_mode() -> String {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(THEME_STORAGE_KEY).ok().flatten())
        .filter(|mode| matches!(mode.as_str(), "light" | "dark" | "system"))
        .unwrap_or_else(|| "system".into())
}

pub(crate) fn apply_theme_mode(mode: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    if let Some(root) = window.document().and_then(|d| d.document_element()) {
        let _ = root.set_attribute("data-theme", mode);
    }
    if let Ok(Some(storage)) = window.local_storage() {
        let _ = storage.set_item(THEME_STORAGE_KEY, mode);
    }
}

fn load_palette_mode(key: &str, fallback: &str, valid: &[&str]) -> String {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(key).ok().flatten())
        .filter(|palette| valid.contains(&palette.as_str()))
        .unwrap_or_else(|| fallback.into())
}

pub(crate) fn load_light_palette() -> String {
    load_palette_mode(
        "wisp-light-palette",
        "paper",
        &["paper", "codex", "github", "catppuccin", "everforest"],
    )
}

pub(crate) fn load_dark_palette() -> String {
    load_palette_mode(
        "wisp-dark-palette",
        "charcoal",
        &["charcoal", "codex", "github", "catppuccin", "gruvbox"],
    )
}

pub(crate) fn apply_palette_modes(light: &str, dark: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    if let Some(root) = window.document().and_then(|d| d.document_element()) {
        let _ = root.set_attribute("data-light-palette", light);
        let _ = root.set_attribute("data-dark-palette", dark);
    }
    if let Ok(Some(storage)) = window.local_storage() {
        let _ = storage.set_item("wisp-light-palette", light);
        let _ = storage.set_item("wisp-dark-palette", dark);
    }
}

/// Load a small persisted view preference (sidebar sort/group), constrained to
/// a known set of values so a stale/garbage localStorage entry can't wedge the UI.
pub(crate) fn load_view_pref(key: &str, fallback: &str, valid: &[&str]) -> String {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(key).ok().flatten())
        .filter(|v| valid.contains(&v.as_str()))
        .unwrap_or_else(|| fallback.into())
}

pub(crate) fn save_view_pref(key: &str, value: &str) {
    if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = s.set_item(key, value);
    }
}

fn load_font_size(key: &str, fallback: u16, min: u16, max: u16) -> u16 {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(key).ok().flatten())
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

pub(crate) fn load_ui_font_size() -> u16 {
    load_font_size("wisp-ui-font-size", 14, 0, 30)
}

pub(crate) fn load_code_font_size() -> u16 {
    load_font_size("wisp-code-font-size", 12, 0, 30)
}

/// A user-chosen font family is substituted into the `--font-ui` /
/// `--font-mono` stacks via `var(--font-user-*)`, so strip anything that could
/// break out of the custom-property value.
fn sanitize_font_family(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|c| !matches!(c, ';' | '{' | '}' | '"' | '\'' | '!' | '(' | ')'))
        .take(100)
        .collect::<String>()
        .trim()
        .to_string()
}

fn load_font_family(key: &str) -> String {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(key).ok().flatten())
        .map(|value| sanitize_font_family(&value))
        .unwrap_or_default()
}

pub(crate) fn load_ui_font_family() -> String {
    load_font_family("wisp-font-ui")
}

pub(crate) fn load_code_font_family() -> String {
    load_font_family("wisp-font-mono")
}

pub(crate) fn apply_font_prefs(ui_size: u16, code_size: u16, ui_family: &str, code_family: &str) {
    let ui_family = sanitize_font_family(ui_family);
    let code_family = sanitize_font_family(code_family);
    let Some(window) = web_sys::window() else {
        return;
    };
    if let Some(root) = window.document().and_then(|d| d.document_element()) {
        let mut style = format!("--ui-font-size:{ui_size}px;--code-font-size:{code_size}px");
        if !ui_family.is_empty() {
            style.push_str(&format!(";--font-user-ui:{ui_family}"));
        }
        if !code_family.is_empty() {
            style.push_str(&format!(";--font-user-mono:{code_family}"));
        }
        let _ = root.set_attribute("style", &style);
    }
    if let Ok(Some(storage)) = window.local_storage() {
        let _ = storage.set_item("wisp-ui-font-size", &ui_size.to_string());
        let _ = storage.set_item("wisp-code-font-size", &code_size.to_string());
        for (key, value) in [("wisp-font-ui", ui_family), ("wisp-font-mono", code_family)] {
            let _ = if value.is_empty() {
                storage.remove_item(key)
            } else {
                storage.set_item(key, &value)
            };
        }
    }
}

pub(crate) const COMPOSER_H_DEFAULT: f64 = 220.0;

pub(crate) const COMPOSER_H_MIN: f64 = 80.0;

pub(crate) const COMPOSER_H_MAX: f64 = 400.0;

pub(crate) const COMPOSER_H_KEY: &str = "composerHeight";

pub(crate) const COMPOSER_H_SAVED_KEY: &str = "composerHeightCustom";

pub(crate) const SIDEBAR_W_DEFAULT: f64 = 248.0;

pub(crate) const SIDEBAR_W_MIN: f64 = 200.0;

pub(crate) const SIDEBAR_W_MAX: f64 = 520.0;

pub(crate) const SIDEBAR_W_KEY: &str = "sidebarWidth";

pub(crate) fn load_composer_h() -> f64 {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(COMPOSER_H_KEY).ok().flatten())
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(COMPOSER_H_DEFAULT)
        .clamp(COMPOSER_H_MIN, COMPOSER_H_MAX)
}

pub(crate) fn composer_h_custom() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(COMPOSER_H_SAVED_KEY).ok().flatten())
        .is_some_and(|v| v == "1")
}

pub(crate) fn save_composer_h(h: f64) {
    if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = s.set_item(COMPOSER_H_KEY, &h.to_string());
        let _ = s.set_item(COMPOSER_H_SAVED_KEY, "1");
    }
}

pub(crate) fn load_sidebar_w() -> f64 {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(SIDEBAR_W_KEY).ok().flatten())
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(SIDEBAR_W_DEFAULT)
        .clamp(SIDEBAR_W_MIN, SIDEBAR_W_MAX)
}

pub(crate) fn save_sidebar_w(w: f64) {
    if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = s.set_item(SIDEBAR_W_KEY, &w.to_string());
    }
}
