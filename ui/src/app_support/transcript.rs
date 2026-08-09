use super::*;

const MAX_IDLE_TRANSCRIPT_CACHE: usize = 8;

fn trim_idle_transcript_cache(
    cache: &mut HashMap<String, Vec<ChatItem>>,
    running: &HashSet<String>,
    protected: Option<&str>,
) {
    let idle_count = cache.keys().filter(|id| !running.contains(*id)).count();
    let idle = cache
        .keys()
        .filter(|id| !running.contains(*id) && protected != Some(id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let remove = idle_count.saturating_sub(MAX_IDLE_TRANSCRIPT_CACHE);
    // ponytail: arbitrary idle eviction is enough because SQLite is the source
    // of truth; add LRU ordering only if reload latency becomes measurable.
    for id in idle.into_iter().take(remove) {
        cache.remove(&id);
    }
}

/// Replace the visible transcript in one signal write, moving the old rows
/// into the inactive-session cache and taking cached target rows by ownership.
pub(crate) fn replace_visible_transcript(
    current_id: Option<String>,
    target_id: Option<&str>,
    fallback: Vec<ChatItem>,
    items: RwSignal<Vec<ChatItem>>,
    transcripts: RwSignal<HashMap<String, Vec<ChatItem>>>,
    running: RwSignal<HashSet<String>>,
) {
    if target_id.is_some() && current_id.as_deref() == target_id {
        return;
    }
    let next = transcripts
        .try_update(|cache| {
            target_id
                .and_then(|id| cache.remove(id))
                .unwrap_or(fallback)
        })
        .unwrap_or_default();
    let previous = items
        .try_update(|visible| std::mem::replace(visible, next))
        .unwrap_or_default();
    let running = running.get_untracked();
    transcripts.update(|cache| {
        if let Some(current_id) = current_id.as_ref() {
            cache.insert(current_id.clone(), previous);
        }
        trim_idle_transcript_cache(cache, &running, current_id.as_deref());
    });
}

#[cfg(test)]
mod transcript_cache_tests {
    use super::{
        replace_visible_transcript, trim_idle_transcript_cache, MAX_IDLE_TRANSCRIPT_CACHE,
    };
    use crate::dto::ChatItem;
    use leptos::{create_runtime, create_rw_signal, SignalGetUntracked};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn trims_only_idle_transcripts() {
        let running = HashSet::from(["running".to_string()]);
        let mut cache =
            HashMap::from([("running".to_string(), vec![ChatItem::User("live".into())])]);
        for index in 0..MAX_IDLE_TRANSCRIPT_CACHE + 3 {
            cache.insert(format!("idle-{index}"), Vec::new());
        }
        trim_idle_transcript_cache(&mut cache, &running, None);
        assert!(cache.contains_key("running"));
        assert_eq!(cache.len(), MAX_IDLE_TRANSCRIPT_CACHE + 1);
    }

    #[test]
    fn moves_rows_between_visible_and_cached_owners() {
        let runtime = create_runtime();
        let items = create_rw_signal(vec![ChatItem::User("session-a".into())]);
        let transcripts = create_rw_signal(HashMap::from([(
            "b".to_string(),
            vec![ChatItem::User("session-b".into())],
        )]));
        let running = create_rw_signal(HashSet::new());

        replace_visible_transcript(
            Some("a".into()),
            Some("b"),
            Vec::new(),
            items,
            transcripts,
            running,
        );

        assert!(matches!(
            items.get_untracked().as_slice(),
            [ChatItem::User(text)] if text == "session-b"
        ));
        let cache = transcripts.get_untracked();
        assert!(!cache.contains_key("b"));
        assert!(matches!(
            cache.get("a").map(Vec::as_slice),
            Some([ChatItem::User(text)]) if text == "session-a"
        ));
        runtime.dispose();
    }
}

/// Map the reviewer's `[msg:N]` index to the live UI row. Usage, reviewer
/// handoffs, approvals, and review cards are UI-only and must not shift it.
pub(crate) fn review_message_ui_index(items: &[ChatItem], message_index: usize) -> Option<usize> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| match item {
            ChatItem::User(text) | ChatItem::Assistant { text, .. } | ChatItem::Reasoning(text) => {
                !text.trim().is_empty()
            }
            ChatItem::Tool { name, .. } => name != "attempt_completion",
            ChatItem::AcpTool { .. } | ChatItem::Plan(_) | ChatItem::Question(_) => true,
            ChatItem::QueuedUser { .. }
            | ChatItem::FileChanged(_)
            | ChatItem::ApprovalPending { .. }
            | ChatItem::AcpPermission { .. }
            | ChatItem::Usage { .. }
            | ChatItem::Compaction { .. }
            | ChatItem::ReviewTransition { .. }
            | ChatItem::Review(_) => false,
        })
        .nth(message_index)
        .map(|(ui_index, _)| ui_index)
}

#[cfg(test)]
mod review_jump_tests {
    use super::{composer_text_from_user_message, review_message_ui_index};
    use crate::dto::{ChatItem, ContextUsage, ReviewTransitionPhase};

    fn assistant(text: &str) -> ChatItem {
        ChatItem::Assistant {
            text: text.into(),
            model: None,
            resources: vec![],
        }
    }

    #[test]
    fn review_indices_ignore_ui_only_rows() {
        let items = vec![
            ChatItem::User("earlier question".into()),
            assistant("earlier answer"),
            ChatItem::Usage {
                input: 10,
                output: 2,
                reasoning: 0,
                cached: 0,
                ctx_tokens: 0,
                max_context: 0,
                context_usage: ContextUsage::default(),
            },
            ChatItem::ReviewTransition {
                phase: ReviewTransitionPhase::Reviewing,
                model: None,
            },
            ChatItem::User("current question".into()),
            assistant("problematic answer"),
        ];

        assert_eq!(review_message_ui_index(&items, 3), Some(5));
    }

    #[test]
    fn editing_a_feedback_turn_excludes_hidden_diagnostics() {
        assert_eq!(
            composer_text_from_user_message(
                "The app froze\n\nFeedback context: \"Wisp version: 0.34.0\""
            ),
            "The app froze"
        );
    }
}

pub(crate) fn composer_text_from_user_message(text: &str) -> String {
    [
        "\n\nUploaded files: ",
        "\n\nAttached artifacts: ",
        "\n\nAttached sessions: ",
        "\n\nProject context: ",
        "\n\nSelected skills: ",
        "\n\nSelected workflows: ",
        "\n\nTarget environments: ",
        "\n\nTarget runtimes: ",
        "\n\nAI source-edit instruction: ",
        "\n\nFeedback context: ",
    ]
    .iter()
    .filter_map(|marker| text.find(marker))
    .min()
    .map(|idx| text[..idx].trim().to_string())
    .unwrap_or_else(|| text.to_string())
}

pub(crate) fn user_message_index(items: &[ChatItem], ui_index: usize) -> Option<usize> {
    if !matches!(items.get(ui_index), Some(ChatItem::User(_))) {
        return None;
    }
    Some(
        items
            .iter()
            .take(ui_index + 1)
            .filter(|item| matches!(item, ChatItem::User(_)))
            .count()
            .saturating_sub(1),
    )
}

pub(crate) fn user_turn_index(items: &[ChatItem], ui_index: usize) -> Option<usize> {
    if !matches!(
        items.get(ui_index),
        Some(ChatItem::User(_) | ChatItem::QueuedUser { .. })
    ) {
        return None;
    }
    Some(
        items
            .iter()
            .take(ui_index + 1)
            .filter(|item| matches!(item, ChatItem::User(_) | ChatItem::QueuedUser { .. }))
            .count()
            .saturating_sub(1),
    )
}

/// Return the stable user-turn index owning any row in that turn. Unlike
/// `user_turn_index`, this intentionally maps assistant/tool rows back to the
/// most recent user row so turn-boundary actions can live on the reply.
pub(crate) fn owning_user_turn_index(items: &[ChatItem], ui_index: usize) -> Option<usize> {
    items
        .iter()
        .take(ui_index + 1)
        .filter(|item| matches!(item, ChatItem::User(_) | ChatItem::QueuedUser { .. }))
        .count()
        .checked_sub(1)
}

pub(crate) fn transcript_item_timestamp(
    items: &[ChatItem],
    ui_index: usize,
    user_offset: usize,
    outline: &[SessionOutlineItem],
) -> Option<i64> {
    let item = items.get(ui_index)?;
    if !matches!(
        item,
        ChatItem::User(_) | ChatItem::QueuedUser { .. } | ChatItem::Assistant { .. }
    ) {
        return None;
    }
    let user_index = items
        .iter()
        .take(ui_index + 1)
        .filter(|item| matches!(item, ChatItem::User(_) | ChatItem::QueuedUser { .. }))
        .count()
        .checked_sub(1)?
        + user_offset;
    let entry = outline
        .iter()
        .find(|entry| entry.user_index == user_index)?;
    match item {
        ChatItem::User(_) | ChatItem::QueuedUser { .. } => {
            entry.sent_at.filter(|timestamp| *timestamp > 0)
        }
        ChatItem::Assistant { .. } => entry.response_at.filter(|timestamp| *timestamp > 0),
        _ => None,
    }
}

pub(crate) fn turn_duration_ms(sent_at: Option<i64>, response_at: Option<i64>) -> Option<u64> {
    let sent_at = sent_at.filter(|timestamp| *timestamp > 0)?;
    let response_at = response_at.filter(|timestamp| *timestamp >= sent_at)?;
    Some(response_at.saturating_sub(sent_at) as u64 * 1_000)
}

pub(crate) fn merge_conversation_outline(
    persisted: &[SessionOutlineItem],
    items: &[ChatItem],
    user_offset: usize,
) -> Vec<SessionOutlineItem> {
    let mut persisted = persisted.to_vec();
    if !persisted
        .windows(2)
        .all(|window| window[0].user_index <= window[1].user_index)
    {
        persisted.sort_by_key(|entry| entry.user_index);
    }
    let live = items
        .iter()
        .filter_map(|item| match item {
            ChatItem::User(text) | ChatItem::QueuedUser { text, .. } => Some(text),
            _ => None,
        })
        .enumerate()
        .map(|(local_index, text)| SessionOutlineItem {
            user_index: user_offset + local_index,
            seq: None,
            text: text.clone(),
            sent_at: None,
            response_at: None,
        })
        .collect::<Vec<_>>();

    // Both inputs are ordered by user index, so merge once instead of finding
    // every live turn in the growing persisted vector and sorting afterwards.
    let mut outline = Vec::with_capacity(persisted.len() + live.len());
    let mut persisted = persisted.into_iter().peekable();
    let mut live = live.into_iter().peekable();
    while let (Some(saved), Some(current)) = (persisted.peek(), live.peek()) {
        match saved.user_index.cmp(&current.user_index) {
            std::cmp::Ordering::Less => outline.push(persisted.next().unwrap()),
            std::cmp::Ordering::Greater => outline.push(live.next().unwrap()),
            std::cmp::Ordering::Equal => {
                let mut saved = persisted.next().unwrap();
                saved.text = live.next().unwrap().text;
                outline.push(saved);
            }
        }
    }
    outline.extend(persisted);
    outline.extend(live);
    outline
}

pub(crate) fn conversation_outline_target_is_loaded(
    items: &[ChatItem],
    user_offset: usize,
    target: usize,
) -> bool {
    let loaded = items
        .iter()
        .filter(|item| matches!(item, ChatItem::User(_) | ChatItem::QueuedUser { .. }))
        .count();
    (user_offset..user_offset + loaded).contains(&target)
}

/// Return a DOM-sized transcript slice without splitting a user turn. A
/// `requested_start` of `usize::MAX` follows the newest available turns.
pub(crate) fn transcript_render_window(
    items: &[ChatItem],
    requested_start: usize,
    max_user_turns: usize,
) -> (std::ops::Range<usize>, usize, usize) {
    let user_rows = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            matches!(item, ChatItem::User(_) | ChatItem::QueuedUser { .. }).then_some(index)
        })
        .collect::<Vec<_>>();
    let total = user_rows.len();
    if total == 0 {
        return (0..items.len(), 0, 0);
    }
    let max_user_turns = max_user_turns.max(1);
    let latest_start = total.saturating_sub(max_user_turns);
    let start = if requested_start == usize::MAX {
        latest_start
    } else {
        requested_start.min(latest_start)
    };
    let end = (start + max_user_turns).min(total);
    let first_item = if start == 0 { 0 } else { user_rows[start] };
    let last_item = if end == total {
        items.len()
    } else {
        user_rows[end]
    };
    (first_item..last_item, start, total)
}

#[cfg(test)]
mod transcript_render_window_tests {
    use super::transcript_render_window;
    use crate::dto::ChatItem;

    #[test]
    fn limits_complete_user_turns_and_can_follow_the_tail() {
        let items = (0..6)
            .flat_map(|turn| {
                [
                    ChatItem::User(format!("question {turn}")),
                    ChatItem::Assistant {
                        text: format!("answer {turn}"),
                        model: None,
                        resources: Vec::new(),
                    },
                ]
            })
            .collect::<Vec<_>>();

        assert_eq!(transcript_render_window(&items, 0, 2), (0..4, 0, 6));
        assert_eq!(
            transcript_render_window(&items, usize::MAX, 2),
            (8..12, 4, 6)
        );
        assert_eq!(transcript_render_window(&items, 2, 2), (4..8, 2, 6));
    }
}

#[cfg(test)]
mod conversation_outline_tests {
    use super::{
        conversation_outline_target_is_loaded, merge_conversation_outline,
        owning_user_turn_index, transcript_item_timestamp, turn_duration_ms, user_turn_index,
    };
    use crate::dto::{ChatItem, SessionOutlineItem};

    #[test]
    fn merges_live_turns_into_the_persisted_directory() {
        let persisted = vec![
            SessionOutlineItem {
                user_index: 0,
                seq: Some(1),
                text: "first".into(),
                sent_at: Some(100),
                response_at: Some(110),
            },
            SessionOutlineItem {
                user_index: 1,
                seq: Some(3),
                text: "stale second".into(),
                sent_at: Some(200),
                response_at: Some(210),
            },
        ];
        let items = vec![
            ChatItem::User("second".into()),
            ChatItem::Assistant {
                text: "answer".into(),
                model: None,
                resources: Vec::new(),
            },
            ChatItem::QueuedUser {
                id: 7,
                text: "third".into(),
            },
        ];

        assert_eq!(
            merge_conversation_outline(&persisted, &items, 1),
            vec![
                persisted[0].clone(),
                SessionOutlineItem {
                    user_index: 1,
                    seq: Some(3),
                    text: "second".into(),
                    sent_at: Some(200),
                    response_at: Some(210),
                },
                SessionOutlineItem {
                    user_index: 2,
                    seq: None,
                    text: "third".into(),
                    sent_at: None,
                    response_at: None,
                },
            ]
        );
        assert_eq!(
            transcript_item_timestamp(&items, 0, 1, &persisted),
            Some(200)
        );
        assert_eq!(
            transcript_item_timestamp(&items, 1, 1, &persisted),
            Some(210)
        );
        assert_eq!(user_turn_index(&items, 2), Some(1));
        assert_eq!(owning_user_turn_index(&items, 1), Some(0));
        assert_eq!(owning_user_turn_index(&items, 2), Some(1));
        assert!(conversation_outline_target_is_loaded(&items, 1, 2));
        assert!(!conversation_outline_target_is_loaded(&items, 1, 0));
    }

    #[test]
    fn turn_duration_uses_the_turn_boundaries() {
        assert_eq!(turn_duration_ms(Some(100), Some(480)), Some(380_000));
        assert_eq!(turn_duration_ms(Some(100), None), None);
        assert_eq!(turn_duration_ms(Some(100), Some(99)), None);
    }

    #[test]
    fn normalizes_an_old_unsorted_directory_before_merging() {
        let persisted = vec![
            SessionOutlineItem {
                user_index: 2,
                seq: Some(5),
                text: "third".into(),
                sent_at: None,
                response_at: None,
            },
            SessionOutlineItem {
                user_index: 0,
                seq: Some(1),
                text: "first".into(),
                sent_at: None,
                response_at: None,
            },
        ];

        let merged = merge_conversation_outline(&persisted, &[ChatItem::User("second".into())], 1);

        assert_eq!(
            merged
                .iter()
                .map(|entry| (entry.user_index, entry.text.as_str()))
                .collect::<Vec<_>>(),
            vec![(0, "first"), (1, "second"), (2, "third")]
        );
    }
}

pub(crate) fn focus_composer() {
    focus_element("composer-input");
}

pub(crate) fn focus_composer_at(caret: u32) {
    focus_composer();
    let Some(textarea) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("composer-input"))
        .and_then(|element| element.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
    else {
        return;
    };
    let _ = textarea.set_selection_range(caret, caret);
}

pub(crate) fn focus_element(id: &str) {
    focus_element_inner(id, false);
}

pub(crate) fn focus_and_select_element(id: &str) {
    focus_element_inner(id, true);
}

fn focus_element_inner(id: &str, select_all: bool) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Some(el) = doc.get_element_by_id(id) else {
        return;
    };
    let _ = el.dyn_ref::<web_sys::HtmlElement>().map(|e| e.focus());
    if !select_all {
        return;
    }
    if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
        input.select();
    } else if let Some(ta) = el.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        ta.select();
    }
}

pub(crate) fn focus_element_soon(id: &'static str) {
    schedule_focus(id, false);
}

/// Focus a text field after the next paint and select its contents.
/// Used by rename/create modals so Ctrl/⌘A and typing work immediately.
pub(crate) fn focus_and_select_soon(id: &'static str) {
    schedule_focus(id, true);
}

fn schedule_focus(id: &'static str, select_all: bool) {
    let focus = Closure::once(move || {
        if select_all {
            focus_and_select_element(id);
        } else {
            focus_element(id);
        }
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            focus.as_ref().unchecked_ref(),
            0,
        );
    }
    focus.forget();
}

pub(crate) fn attachment_name(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}
