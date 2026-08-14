//! `/share` support: turn the transcript into a selectable, redactable list
//! of messages that the share overlay renders into a long PNG.

use crate::dto::ChatItem;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShareRole {
    User,
    Assistant,
    Thinking,
}

impl ShareRole {
    /// Stable wire tag consumed by the JS canvas renderer.
    pub(crate) fn tag(self) -> &'static str {
        match self {
            ShareRole::User => "user",
            ShareRole::Assistant => "assistant",
            ShareRole::Thinking => "thinking",
        }
    }
}

#[derive(Clone)]
pub(crate) struct ShareMessage {
    pub(crate) role: ShareRole,
    pub(crate) text: String,
    pub(crate) selected: bool,
}

/// Build the exportable message list from the transcript. Only conversation
/// content is shareable (user, assistant, and thinking rows); tool calls,
/// usage rows, and other machinery never appear in the share image. Thinking
/// rows are present but deselected — hidden from the export by default.
pub(crate) fn share_messages(items: &[ChatItem]) -> Vec<ShareMessage> {
    items
        .iter()
        .filter_map(|item| match item {
            ChatItem::User(text) => Some((ShareRole::User, text.as_str(), true)),
            ChatItem::Assistant { text, .. } => Some((ShareRole::Assistant, text.as_str(), true)),
            ChatItem::Reasoning(text) => Some((ShareRole::Thinking, text.as_str(), false)),
            _ => None,
        })
        .filter(|(_, text, _)| !text.trim().is_empty())
        .map(|(role, text, selected)| ShareMessage {
            role,
            text: text.trim().to_string(),
            selected,
        })
        .collect()
}

/// Split the redaction input into keywords: comma (ASCII or fullwidth) and
/// newline separated, trimmed, deduplicated, longest first so a keyword is
/// never partially masked by one of its own substrings.
pub(crate) fn parse_redact_keywords(raw: &str) -> Vec<String> {
    let mut keywords: Vec<String> = raw
        .split(|c| c == ',' || c == '，' || c == '\n')
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .map(str::to_string)
        .collect();
    keywords.sort_by_key(|k| std::cmp::Reverse(k.chars().count()));
    keywords.dedup();
    let mut seen = std::collections::HashSet::new();
    keywords.retain(|k| seen.insert(k.clone()));
    keywords
}

/// Replace every (case-insensitive) occurrence of each keyword with `xxx`.
pub(crate) fn redact_text(text: &str, keywords: &[String]) -> String {
    keywords
        .iter()
        .filter(|k| !k.is_empty())
        .fold(text.to_string(), |acc, keyword| {
            replace_ci(&acc, keyword, "xxx")
        })
}

/// Case-insensitive literal replacement. Matches on full Unicode lowercase
/// equality so `Alice`, `ALICE`, and `alice` all mask with one keyword.
fn replace_ci(haystack: &str, needle: &str, replacement: &str) -> String {
    let needle_lower: Vec<char> = needle.chars().flat_map(char::to_lowercase).collect();
    if needle_lower.is_empty() {
        return haystack.to_string();
    }
    let mut out = String::with_capacity(haystack.len());
    let mut rest = haystack;
    'outer: while !rest.is_empty() {
        if let Some(len) = ci_prefix_len(rest, &needle_lower) {
            out.push_str(replacement);
            rest = &rest[len..];
            continue 'outer;
        }
        let ch = rest.chars().next().unwrap();
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

/// Byte length of a prefix of `text` whose lowercase form equals
/// `needle_lower`, or `None` when `text` does not start with the needle.
fn ci_prefix_len(text: &str, needle_lower: &[char]) -> Option<usize> {
    let mut expected = needle_lower.iter();
    let mut len = 0;
    for ch in text.chars() {
        for lower in ch.to_lowercase() {
            if expected.next() != Some(&lower) {
                return None;
            }
        }
        len += ch.len_utf8();
        if expected.len() == 0 {
            return Some(len);
        }
    }
    None
}

#[cfg(test)]
mod share_tests {
    use super::*;

    fn assistant(text: &str) -> ChatItem {
        ChatItem::Assistant {
            text: text.into(),
            model: None,
            resources: vec![],
        }
    }

    #[test]
    fn builds_shareable_rows_with_thinking_deselected() {
        let items = vec![
            ChatItem::User("检查这个峰".into()),
            ChatItem::Reasoning("先比对参考谱库".into()),
            ChatItem::Tool {
                name: "shell".into(),
                ok: Some(true),
                input: "ls".into(),
                output: "ok".into(),
                started_at_ms: None,
                duration_ms: None,
            },
            assistant("这是主峰的解释。"),
            ChatItem::Reasoning("   ".into()),
            assistant(""),
        ];
        let rows = share_messages(&items);
        assert_eq!(rows.len(), 3);
        assert!(matches!(rows[0].role, ShareRole::User) && rows[0].selected);
        assert!(matches!(rows[1].role, ShareRole::Thinking) && !rows[1].selected);
        assert!(matches!(rows[2].role, ShareRole::Assistant) && rows[2].selected);
    }

    #[test]
    fn parses_keywords_longest_first_without_duplicates() {
        let keywords = parse_redact_keywords("  alice , bob，alice\nalice smith,, ");
        assert_eq!(keywords, vec!["alice smith", "alice", "bob"]);
    }

    #[test]
    fn redacts_case_insensitively_and_in_cjk_text() {
        let keywords = parse_redact_keywords("Alice,张三");
        assert_eq!(
            redact_text("ALICE told alice about 张三 and Alicete", &keywords),
            "xxx told xxx about xxx and xxxte"
        );
        assert_eq!(redact_text("张三丰不是张三", &keywords), "xxx丰不是xxx");
    }

    #[test]
    fn longer_keywords_mask_before_their_substrings() {
        let keywords = parse_redact_keywords("alice smith, alice");
        assert_eq!(
            redact_text("alice smith met alice", &keywords),
            "xxx met xxx"
        );
    }

    #[test]
    fn empty_keywords_leave_text_untouched() {
        assert_eq!(redact_text("nothing to hide", &[]), "nothing to hide");
        assert!(parse_redact_keywords(" ,，\n").is_empty());
    }
}
