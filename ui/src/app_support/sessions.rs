use super::*;

pub(crate) fn refresh_sessions(
    sessions: RwSignal<Vec<SessionInfo>>,
    pending: RwSignal<HashMap<String, usize>>,
    running: RwSignal<HashSet<String>>,
    next_cursor: RwSignal<Option<SessionCursor>>,
    active_session: RwSignal<Option<String>>,
) {
    next_cursor.set(None);
    spawn_local(async move {
        let args = to_value(&serde_json::json!({ "cursor": null })).unwrap();
        let v = invoke("list_sessions_page", args).await;
        if let Ok(mut page) = serde_wasm_bindgen::from_value::<SessionPage>(v) {
            let set = pending.with_untracked(|m| rebuilt_running_set(&page.running_ids, m));
            running.set(set);
            let active = active_session.get_untracked();
            let active_is_listed = active.as_ref().is_none_or(|id| {
                page.items
                    .iter()
                    .any(|session| session.id.as_str() == id.as_str())
            });
            if !active_is_listed {
                let id = active.expect("an unlisted active session has an id");
                let draft = sessions
                    .with_untracked(|current| {
                        current.iter().find(|session| session.id == id).cloned()
                    })
                    .unwrap_or(SessionInfo {
                        id,
                        title: String::new(),
                        ts: js_sys::Date::now() as i64,
                        folder_id: None,
                        branched_from: None,
                        pinned: false,
                        branch_state: None,
                        stale_prompt: false,
                    });
                page.items.insert(0, draft);
            }
            sessions.set(page.items);
            next_cursor.set(page.next_cursor);
        }
    });
}

pub(crate) fn load_older_sessions(
    sessions: RwSignal<Vec<SessionInfo>>,
    pending: RwSignal<HashMap<String, usize>>,
    running: RwSignal<HashSet<String>>,
    next_cursor: RwSignal<Option<SessionCursor>>,
    loading: RwSignal<bool>,
) {
    let Some(cursor) = next_cursor.get_untracked() else {
        return;
    };
    loading.set(true);
    spawn_local(async move {
        let args = to_value(&serde_json::json!({ "cursor": cursor })).unwrap();
        let v = invoke("list_sessions_page", args).await;
        if let Ok(page) = serde_wasm_bindgen::from_value::<SessionPage>(v) {
            let set = pending.with_untracked(|m| rebuilt_running_set(&page.running_ids, m));
            running.set(set);
            sessions.update(|current| {
                let existing = current
                    .iter()
                    .map(|item| item.id.clone())
                    .collect::<HashSet<_>>();
                current.extend(
                    page.items
                        .into_iter()
                        .filter(|item| !existing.contains(&item.id)),
                );
            });
            next_cursor.set(page.next_cursor);
        }
        loading.set(false);
    });
}

/// Rebuild the local `running` set from the backend's session-page snapshot
/// so restarts, project switches and other windows' turns are reflected. Keeps
/// locally pending sends the backend may not have registered yet.
pub(crate) fn rebuilt_running_set(
    running_ids: &[String],
    pending: &HashMap<String, usize>,
) -> HashSet<String> {
    let mut set: HashSet<String> = running_ids.iter().cloned().collect();
    set.extend(pending.keys().cloned());
    set
}

#[cfg(test)]
mod rebuilt_running_set_tests {
    use super::*;

    #[test]
    fn keeps_server_running_and_local_pending() {
        let running = vec!["a".to_string()];
        let pending = HashMap::from([("c".to_string(), 1)]);
        let set = rebuilt_running_set(&running, &pending);
        assert!(set.contains("a"), "server-running kept");
        assert!(!set.contains("b"), "stale local state dropped");
        assert!(set.contains("c"), "local pending send kept");
    }
}

pub(crate) fn refresh_folders(folders: RwSignal<Vec<FolderInfo>>) {
    spawn_local(async move {
        let v = invoke("list_folders", JsValue::UNDEFINED).await;
        if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<FolderInfo>>(v) {
            folders.set(list);
        }
    });
}

pub(crate) fn refresh_explorations(explorations: RwSignal<Vec<ExplorationSummary>>) {
    spawn_local(async move {
        let value = invoke("list_project_explorations", JsValue::UNDEFINED).await;
        if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<ExplorationSummary>>(value) {
            explorations.set(list);
        }
    });
}


/// Split the sidebar list into top-level sessions plus, per session id, the
/// branches forked from it — the sidebar draws those nested underneath (#531).
///
/// A branch only nests when its source is in the same visible list *and* the
/// same folder; otherwise it stays top-level, so nothing silently disappears
/// when the source is on an older page or was dragged elsewhere. Branches of
/// branches flatten onto the topmost visible source: one level of indent only.
pub(crate) fn nest_branch_sessions(
    list: &[SessionInfo],
) -> (Vec<SessionInfo>, HashMap<String, Vec<SessionInfo>>) {
    let by_id: HashMap<&str, &SessionInfo> = list.iter().map(|s| (s.id.as_str(), s)).collect();
    let visible_source = |s: &SessionInfo| -> Option<String> {
        let mut cur = s;
        let mut hops = 0;
        loop {
            let Some(src) = cur.branched_from.as_deref().and_then(|id| by_id.get(id)) else {
                break;
            };
            if src.folder_id != cur.folder_id {
                return None;
            }
            cur = src;
            hops += 1;
            // ponytail: hop cap is the cycle guard; real branch chains are short.
            if hops > 16 {
                return None;
            }
        }
        (cur.id != s.id).then(|| cur.id.clone())
    };
    let mut top = Vec::new();
    let mut branches: HashMap<String, Vec<SessionInfo>> = HashMap::new();
    for s in list {
        match visible_source(s) {
            Some(source) => branches.entry(source).or_default().push(s.clone()),
            None => top.push(s.clone()),
        }
    }
    (top, branches)
}

#[cfg(test)]
mod branch_nesting_tests {
    use super::*;

    fn ses(id: &str, branched_from: Option<&str>, folder: Option<&str>) -> SessionInfo {
        SessionInfo {
            id: id.into(),
            title: id.into(),
            ts: 0,
            folder_id: folder.map(Into::into),
            branched_from: branched_from.map(Into::into),
            pinned: false,
            branch_state: branched_from.map(|_| "active".into()),
        }
    }

    #[test]
    fn branches_nest_under_their_visible_source() {
        let list = vec![
            ses("main", None, None),
            ses("b1", Some("main"), None),
            ses("b2", Some("b1"), None),
            ses("other", None, None),
        ];
        let (top, branches) = nest_branch_sessions(&list);
        assert_eq!(
            top.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["main", "other"]
        );
        // Branch-of-a-branch flattens onto the topmost visible source.
        assert_eq!(
            branches["main"]
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>(),
            ["b1", "b2"]
        );
    }

    #[test]
    fn branch_stays_top_level_when_source_is_absent_or_elsewhere() {
        let list = vec![
            ses("orphan", Some("gone"), None),
            ses("main", None, None),
            ses("moved", Some("main"), Some("f1")),
            ses("cycle-a", Some("cycle-b"), None),
            ses("cycle-b", Some("cycle-a"), None),
        ];
        let (top, branches) = nest_branch_sessions(&list);
        assert!(branches.is_empty());
        assert_eq!(top.len(), list.len());
    }
}

pub(crate) fn bucket_sessions_by_date(
    list: &[SessionInfo],
) -> (Vec<SessionInfo>, Vec<SessionInfo>) {
    let now_ms = js_sys::Date::now();
    let (mut today, mut earlier) = (Vec::new(), Vec::new());
    for s in list {
        let ts_ms = if s.ts > 1_000_000_000_000 {
            s.ts as f64
        } else {
            s.ts as f64 * 1000.0
        };
        if s.ts > 0 && ts_ms >= now_ms - 86_400_000.0 {
            today.push(s.clone());
        } else {
            earlier.push(s.clone());
        }
    }
    (today, earlier)
}
