use super::*;

pub(crate) fn refresh_dir(cwd: RwSignal<String>, entries: RwSignal<Vec<DirEntry>>) {
    spawn_local(async move {
        let path = cwd.get();
        let v = invoke(
            "list_dir",
            to_value(&serde_json::json!({ "path": path })).unwrap(),
        )
        .await;
        if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<DirEntry>>(v) {
            entries.set(list);
        }
    });
}

pub(crate) fn refresh_remote_dir(
    context_id: String,
    cwd: RwSignal<String>,
    entries: RwSignal<Vec<DirEntry>>,
    loading: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    active_source: RwSignal<String>,
) {
    let requested_path = cwd.get_untracked();
    entries.set(vec![]);
    error.set(None);
    loading.set(true);
    spawn_local(async move {
        let result = invoke_checked(
            "list_remote_dir",
            to_value(&serde_json::json!({
                "contextId": context_id.clone(),
                "path": requested_path.clone(),
            }))
            .unwrap(),
        )
        .await;
        if active_source.get_untracked() != context_id || cwd.get_untracked() != requested_path {
            return;
        }
        loading.set(false);
        match result {
            Ok(value) => match serde_wasm_bindgen::from_value::<DirectoryListing>(value) {
                Ok(listing) => {
                    cwd.set(listing.path);
                    entries.set(listing.entries);
                }
                Err(parse_error) => error.set(Some(parse_error.to_string())),
            },
            Err(invoke_error) => error.set(Some(js_error_text(invoke_error))),
        }
    });
}

pub(crate) fn refresh_active_file_dir(
    source: RwSignal<String>,
    local_cwd: RwSignal<String>,
    local_entries: RwSignal<Vec<DirEntry>>,
    remote_cwd: RwSignal<String>,
    remote_entries: RwSignal<Vec<DirEntry>>,
    remote_loading: RwSignal<bool>,
    remote_error: RwSignal<Option<String>>,
) {
    let context_id = source.get_untracked();
    if context_id == "local" {
        refresh_dir(local_cwd, local_entries);
    } else {
        refresh_remote_dir(
            context_id,
            remote_cwd,
            remote_entries,
            remote_loading,
            remote_error,
            source,
        );
    }
}

pub(crate) fn refresh_file_search(query: RwSignal<String>, hits: RwSignal<Vec<FileSearchHit>>) {
    spawn_local(async move {
        let q = query.get().trim().to_string();
        if q.is_empty() {
            hits.set(vec![]);
            return;
        }
        let v = invoke(
            "search_files",
            to_value(&serde_json::json!({ "query": q, "limit": 200 })).unwrap(),
        )
        .await;
        if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<FileSearchHit>>(v) {
            hits.set(list);
        }
    });
}

pub(crate) fn refresh_artifact_search(query: RwSignal<String>, hits: RwSignal<Vec<ArtifactInfo>>) {
    spawn_local(async move {
        let q = query.get().trim().to_string();
        let v = invoke(
            "search_artifacts",
            to_value(&serde_json::json!({ "query": q, "limit": 12 })).unwrap(),
        )
        .await;
        if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<ArtifactInfo>>(v) {
            hits.set(list);
        }
    });
}

pub(crate) fn artifact_badge(kind: &str, name: &str) -> String {
    let raw = name
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .filter(|ext| !ext.is_empty() && ext.len() <= 10)
        .or_else(|| kind.rsplit('/').next())
        .unwrap_or("file");
    raw.to_uppercase()
}

pub(crate) fn stored_artifact_path(path: &str) -> String {
    path.strip_prefix("file://").unwrap_or(path).to_string()
}

pub(crate) fn contains_search(q: &str, parts: &[&str]) -> bool {
    q.is_empty() || parts.iter().any(|s| s.to_lowercase().contains(q))
}

pub(crate) type ModalArtifact = (String, String, String);

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CenterFileTab {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) kind: String,
}

impl CenterFileTab {
    pub(crate) fn new(path: String, name: String, kind: String) -> Self {
        Self { path, name, kind }
    }

    pub(crate) fn from_path(path: String) -> Self {
        let name = path.rsplit(['/', '\\']).next().unwrap_or(&path).to_string();
        let kind = file_kind(&path).unwrap_or("text").to_string();
        Self { path, name, kind }
    }
}

/// Keys a live file-change event can use to address an open center tab. Tools
/// may report either the relative argument the model supplied or the resolved
/// absolute path; normalize both POSIX and Windows separators for matching.
pub(crate) fn file_change_refresh_keys(path: &str, project_root: Option<&str>) -> Vec<String> {
    let mut keys = Vec::new();
    let mut push = |value: String| {
        if !value.is_empty() && !keys.contains(&value) {
            keys.push(value);
        }
    };
    push(path.to_string());
    let normalized = path.replace('\\', "/");
    push(normalized.clone());
    if let Some(relative) = normalized.strip_prefix("./") {
        push(relative.to_string());
        push(relative.replace('/', "\\"));
    }
    if let Some(root) = project_root {
        let normalized_root = root.replace('\\', "/");
        let normalized_root = normalized_root.trim_end_matches('/');
        if let Some(relative) = normalized.strip_prefix(normalized_root).and_then(|tail| {
            tail.strip_prefix('/')
                .filter(|relative| !relative.is_empty())
        }) {
            push(relative.to_string());
            push(relative.replace('/', "\\"));
        }
    }
    keys
}

#[cfg(test)]
mod file_change_refresh_keys_tests {
    use super::file_change_refresh_keys;

    #[test]
    fn matches_relative_and_absolute_workspace_paths() {
        assert_eq!(
            file_change_refresh_keys("analysis.R", Some("/work/project")),
            ["analysis.R"]
        );
        assert!(
            file_change_refresh_keys("./analysis.R", Some("/work/project"))
                .contains(&"analysis.R".to_string())
        );
        let unix = file_change_refresh_keys("/work/project/src/analysis.R", Some("/work/project"));
        assert!(unix.contains(&"src/analysis.R".to_string()));

        let windows =
            file_change_refresh_keys(r"C:\work\project\src\analysis.R", Some(r"C:\work\project"));
        assert!(windows.contains(&"src/analysis.R".to_string()));
        assert!(windows.contains(&r"src\analysis.R".to_string()));
    }
}

pub(crate) fn open_workspace_file(path: String, modal_artifact: RwSignal<Option<ModalArtifact>>) {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(&path).to_string();
    let kind = file_kind(&path).unwrap_or("text").to_string();
    modal_artifact.set(Some((path, name, kind)));
}

pub(crate) fn modal_image_nav_targets(
    artifacts: &[Artifact],
    current_path: &str,
    current_kind: &str,
) -> (Option<ModalArtifact>, Option<ModalArtifact>) {
    if current_kind != "image" {
        return (None, None);
    }
    let images = artifacts
        .iter()
        .filter_map(|artifact| match &artifact.data {
            PreviewData::File { path, kind } if kind == "image" => {
                Some((path.clone(), artifact.name.clone(), kind.clone()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let Some(index) = images.iter().position(|(path, _, _)| path == current_path) else {
        return (None, None);
    };
    let prev = index
        .checked_sub(1)
        .and_then(|idx| images.get(idx).cloned());
    let next = images.get(index + 1).cloned();
    (prev, next)
}

pub(crate) const ALL_RIGHT_TABS: [RightTab; 8] = [
    RightTab::Artifacts,
    RightTab::Agents,
    RightTab::Notebook,
    RightTab::Highlights,
    RightTab::File,
    RightTab::Provenance,
    RightTab::Hosts,
    RightTab::SideChat,
];

pub(crate) const DEFAULT_RIGHT_TABS: [RightTab; 4] = [
    RightTab::Artifacts,
    RightTab::Agents,
    RightTab::File,
    RightTab::Hosts,
];

pub(crate) fn scroll_active_right_tab_into_view() {
    request_animation_frame(|| {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let Ok(Some(scroller)) = document.query_selector(".rightpane .rp-tab-scroll") else {
            return;
        };
        let Ok(Some(active)) = document.query_selector(".rightpane .rp-tab.active") else {
            return;
        };
        let Some(active_wrap) = active.parent_element() else {
            return;
        };
        let Ok(scroller) = scroller.dyn_into::<web_sys::HtmlElement>() else {
            return;
        };
        let Ok(active_wrap) = active_wrap.dyn_into::<web_sys::HtmlElement>() else {
            return;
        };

        let tab_left = active_wrap.offset_left();
        let tab_right = tab_left + active_wrap.offset_width();
        let viewport_left = scroller.scroll_left();
        let viewport_width = scroller.client_width();
        let viewport_right = viewport_left + viewport_width;
        if tab_left < viewport_left {
            scroller.set_scroll_left((tab_left - 4).max(0));
        } else if tab_right > viewport_right {
            scroller.set_scroll_left(tab_right - viewport_width + 4);
        }
    });
}

pub(crate) fn ensure_right_tab(
    tab: RightTab,
    show_right: RwSignal<bool>,
    open_right_tabs: RwSignal<Vec<RightTab>>,
    right_tab: RwSignal<RightTab>,
) {
    show_right.set(true);
    open_right_tabs.update(|tabs| {
        if !tabs.iter().any(|t| *t == tab) {
            tabs.push(tab);
        }
    });
    right_tab.set(tab);
}

pub(crate) fn close_right_tab(
    tab: RightTab,
    show_right: RwSignal<bool>,
    open_right_tabs: RwSignal<Vec<RightTab>>,
    right_tab: RwSignal<RightTab>,
) {
    let was_active = right_tab.get_untracked() == tab;
    let prev_idx = open_right_tabs
        .get_untracked()
        .iter()
        .position(|t| *t == tab);
    open_right_tabs.update(|tabs| tabs.retain(|t| *t != tab));
    let remaining = open_right_tabs.get_untracked();
    if remaining.is_empty() {
        show_right.set(false);
        return;
    }
    if was_active {
        let pick = prev_idx
            .map(|i| if i > 0 { i - 1 } else { 0 })
            .unwrap_or(0)
            .min(remaining.len() - 1);
        right_tab.set(remaining[pick]);
    }
}

pub(crate) fn reveal_in_files(
    path: &str,
    file_source: RwSignal<String>,
    file_cwd: RwSignal<String>,
    file_query: RwSignal<String>,
    file_entries: RwSignal<Vec<DirEntry>>,
    show_right: RwSignal<bool>,
    open_right_tabs: RwSignal<Vec<RightTab>>,
    right_tab: RwSignal<RightTab>,
) {
    file_source.set("local".into());
    file_query.set(String::new());
    file_cwd.set(parent_path(path));
    refresh_dir(file_cwd, file_entries);
    ensure_right_tab(RightTab::File, show_right, open_right_tabs, right_tab);
}

pub(crate) fn file_dir_label(path: &str) -> String {
    let p = parent_path(path);
    if p == "." {
        String::new()
    } else {
        format!("{p}/")
    }
}
