//! `/share` support: turn the transcript into a selectable, redactable list
//! of messages that the share overlay renders into a long PNG.

use crate::dto::ChatItem;
use serde_json::{json, Value};

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

// --- Markdown → canvas blocks ----------------------------------------------

/// Inline style flags carried by each text run. The JS canvas renderer maps
/// them to font weight/style/family and link color.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct InlineStyle {
    bold: bool,
    italic: bool,
    code: bool,
    link: bool,
}

/// Which container block is currently collecting inline runs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CurrentBlock {
    Heading(u8),
    Paragraph,
    Item { index: Option<u64> },
}

/// GFM table being flattened into monospace lines (`cell | cell` per row).
struct TableState {
    lines: Vec<String>,
    row: Vec<String>,
    cell: String,
}

/// Parse assistant Markdown into the flat block list the JS canvas renderer
/// draws: headings, paragraphs, list items, quotes, code blocks, tables
/// (flattened to monospace lines) and horizontal rules, with inline runs
/// styled bold/italic/code/link. Mirrors the options chat rendering uses so
/// the exported image matches what the user saw.
pub(crate) fn share_markdown_blocks(text: &str) -> Vec<Value> {
    use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_MATH);

    let mut blocks: Vec<Value> = vec![];
    let mut runs: Vec<(String, InlineStyle)> = vec![];
    let mut style = InlineStyle::default();
    let mut current: Option<CurrentBlock> = None;
    // Tight lists omit Paragraph tags; loose lists nest them inside items.
    // A Paragraph start while an item is open is structural, not a new block.
    let mut item_para = false;
    let mut code_buf: Option<String> = None;
    let mut quote_depth = 0usize;
    let mut lists: Vec<Option<u64>> = vec![];
    let mut table: Option<TableState> = None;
    let mut image_alt: Option<String> = None;

    let push_run = |runs: &mut Vec<(String, InlineStyle)>, text: &str, style: InlineStyle| {
        if text.is_empty() {
            return;
        }
        if let Some((last_text, last_style)) = runs.last_mut() {
            if *last_style == style {
                last_text.push_str(text);
                return;
            }
        }
        runs.push((text.to_string(), style));
    };

    macro_rules! flush_runs {
        () => {{
            let value = runs_to_json(&runs);
            runs.clear();
            value
        }};
    }

    for event in Parser::new_ext(text, opts) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    if matches!(current, Some(CurrentBlock::Item { .. })) {
                        item_para = true;
                    } else {
                        current = Some(CurrentBlock::Paragraph);
                    }
                }
                Tag::Heading { level, .. } => {
                    let level = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        _ => 3,
                    };
                    current = Some(CurrentBlock::Heading(level));
                }
                Tag::BlockQuote(..) => quote_depth += 1,
                Tag::CodeBlock(_) => code_buf = Some(String::new()),
                Tag::List(start) => lists.push(start),
                Tag::Item => {
                    let index = match lists.last_mut() {
                        Some(Some(next)) => {
                            let i = *next;
                            *next += 1;
                            Some(i)
                        }
                        _ => None,
                    };
                    current = Some(CurrentBlock::Item { index });
                }
                Tag::Strong => style.bold = true,
                Tag::Emphasis => style.italic = true,
                Tag::Link { .. } => style.link = true,
                Tag::Image { .. } => image_alt = Some(String::new()),
                Tag::Table(_) => {
                    table = Some(TableState {
                        lines: vec![],
                        row: vec![],
                        cell: String::new(),
                    })
                }
                Tag::TableRow | Tag::TableHead => {
                    if let Some(t) = table.as_mut() {
                        t.row.clear();
                    }
                }
                Tag::TableCell => {
                    if let Some(t) = table.as_mut() {
                        t.cell.clear();
                    }
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => {
                    if item_para {
                        item_para = false;
                    } else if matches!(current, Some(CurrentBlock::Paragraph)) {
                        let value = flush_runs!();
                        if !value.is_empty() {
                            blocks.push(json!({"t": "p", "quote": quote_depth > 0, "runs": value}));
                        }
                        current = None;
                    }
                }
                TagEnd::Heading(_) => {
                    if let Some(CurrentBlock::Heading(level)) = current {
                        let value = flush_runs!();
                        if !value.is_empty() {
                            blocks.push(json!({"t": "h", "level": level, "runs": value}));
                        }
                        current = None;
                    }
                }
                TagEnd::BlockQuote(..) => quote_depth = quote_depth.saturating_sub(1),
                TagEnd::CodeBlock => {
                    if let Some(buf) = code_buf.take() {
                        blocks.push(json!({"t": "code", "text": buf.trim_end_matches('\n')}));
                    }
                }
                TagEnd::List(_) => {
                    lists.pop();
                }
                TagEnd::Item => {
                    if let Some(CurrentBlock::Item { index }) = current {
                        let value = flush_runs!();
                        if !value.is_empty() {
                            blocks.push(json!({
                                "t": "li",
                                "ordered": index.is_some(),
                                "index": index.unwrap_or(0),
                                "depth": lists.len(),
                                "quote": quote_depth > 0,
                                "runs": value,
                            }));
                        }
                        current = None;
                    }
                }
                TagEnd::Strong => style.bold = false,
                TagEnd::Emphasis => style.italic = false,
                TagEnd::Link => style.link = false,
                TagEnd::Image => {
                    if let Some(alt) = image_alt.take() {
                        let label = if alt.trim().is_empty() {
                            "[image]".to_string()
                        } else {
                            format!("[{}]", alt.trim())
                        };
                        push_run(
                            &mut runs,
                            &label,
                            InlineStyle {
                                code: true,
                                ..style
                            },
                        );
                    }
                }
                TagEnd::Table => {
                    if let Some(t) = table.take() {
                        blocks.push(json!({"t": "code", "text": t.lines.join("\n")}));
                    }
                }
                TagEnd::TableRow | TagEnd::TableHead => {
                    if let Some(t) = table.as_mut() {
                        t.lines.push(t.row.join(" | "));
                    }
                }
                TagEnd::TableCell => {
                    if let Some(t) = table.as_mut() {
                        let cell = t.cell.trim().to_string();
                        t.row.push(cell);
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                if let Some(alt) = image_alt.as_mut() {
                    alt.push_str(&text);
                } else if let Some(t) = table.as_mut() {
                    t.cell.push_str(&text);
                } else if let Some(buf) = code_buf.as_mut() {
                    buf.push_str(&text);
                } else {
                    push_run(&mut runs, &text, style);
                }
            }
            Event::Code(text) => {
                if let Some(t) = table.as_mut() {
                    t.cell.push_str(&text);
                } else {
                    push_run(
                        &mut runs,
                        &text,
                        InlineStyle {
                            code: true,
                            ..style
                        },
                    );
                }
            }
            Event::InlineMath(text) => {
                push_run(
                    &mut runs,
                    &text,
                    InlineStyle {
                        code: true,
                        ..style
                    },
                );
            }
            Event::DisplayMath(text) => {
                if current.is_some() {
                    // `$$...$$` inside a paragraph stays inline.
                    push_run(
                        &mut runs,
                        &text,
                        InlineStyle {
                            code: true,
                            ..style
                        },
                    );
                } else {
                    // Display math is its own block (no Paragraph wrapper), so
                    // it must not leak into the inline run buffer.
                    blocks.push(json!({"t": "code", "text": text.as_ref()}));
                }
            }
            Event::SoftBreak => push_run(&mut runs, " ", style),
            Event::HardBreak => push_run(&mut runs, "\n", style),
            Event::Rule => blocks.push(json!({"t": "hr"})),
            Event::TaskListMarker(checked) => {
                push_run(&mut runs, if checked { "☑ " } else { "☐ " }, style);
            }
            Event::FootnoteReference(name) => {
                push_run(&mut runs, &format!("[{name}]"), style);
            }
            _ => {}
        }
    }
    blocks
}

/// Serialize inline runs, trimming surrounding whitespace and dropping runs
/// that carry no visible text.
fn runs_to_json(runs: &[(String, InlineStyle)]) -> Vec<Value> {
    let mut out: Vec<Value> = runs
        .iter()
        .map(|(text, style)| {
            let mut run = json!({"text": text});
            if style.bold {
                run["b"] = json!(true);
            }
            if style.italic {
                run["i"] = json!(true);
            }
            if style.code {
                run["c"] = json!(true);
            }
            if style.link {
                run["a"] = json!(true);
            }
            run
        })
        .collect();
    // Trim leading/trailing whitespace so bubbles do not start or end with a
    // stray space from source formatting.
    if let Some(first) = out.first_mut() {
        if let Some(text) = first["text"].as_str() {
            first["text"] = json!(text.trim_start());
        }
    }
    if let Some(last) = out.last_mut() {
        if let Some(text) = last["text"].as_str() {
            last["text"] = json!(text.trim_end());
        }
    }
    out.retain(|run| !run["text"].as_str().unwrap_or("").is_empty());
    out
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

    #[test]
    fn markdown_blocks_cover_headings_styles_and_lists() {
        let blocks = share_markdown_blocks(
            "## 拟合结果\n\n**主峰** 在 *530 nm*，见 `fit()` 和 [报告](https://x)。\n\n1. 第一步\n2. 第二步\n\n- 备注\n",
        );
        assert_eq!(blocks[0]["t"], json!("h"));
        assert_eq!(blocks[0]["level"], json!(2));
        let runs = blocks[1]["runs"].as_array().unwrap();
        assert_eq!(runs[0], json!({"text": "主峰", "b": true}));
        assert_eq!(runs[2], json!({"text": "530 nm", "i": true}));
        assert!(runs
            .iter()
            .any(|r| r["c"] == json!(true) && r["text"] == "fit()"));
        assert!(runs
            .iter()
            .any(|r| r["a"] == json!(true) && r["text"] == "报告"));
        assert_eq!(blocks[2]["t"], json!("li"));
        assert_eq!(blocks[2]["ordered"], json!(true));
        assert_eq!(blocks[2]["index"], json!(1));
        assert_eq!(blocks[3]["index"], json!(2));
        assert_eq!(blocks[4]["ordered"], json!(false));
    }

    #[test]
    fn markdown_blocks_flatten_code_quotes_rules_and_tables() {
        let blocks = share_markdown_blocks(
            "> 引用一句\n\n```python\nfit(x)\n```\n\n---\n\n| a | b |\n|---|---|\n| 1 | 2 |\n",
        );
        assert_eq!(
            blocks[0],
            json!({"t": "p", "quote": true, "runs": [{"text": "引用一句"}]})
        );
        assert_eq!(blocks[1], json!({"t": "code", "text": "fit(x)"}));
        assert_eq!(blocks[2], json!({"t": "hr"}));
        assert_eq!(blocks[3]["t"], json!("code"));
        assert_eq!(blocks[3]["text"], json!("a | b\n1 | 2"));
    }

    #[test]
    fn markdown_blocks_keep_hard_breaks_and_task_markers() {
        let blocks = share_markdown_blocks("第一行  \n第二行\n\n- [x] 已完成\n- [ ] 待办\n");
        let texts: Vec<&str> = blocks[0]["runs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["text"].as_str().unwrap())
            .collect();
        // Same-style runs merge; the JS wrapper still honors the embedded \n.
        assert_eq!(texts, vec!["第一行\n第二行"]);
        assert_eq!(blocks[1]["runs"][0]["text"], json!("☑ 已完成"));
        assert_eq!(blocks[2]["runs"][0]["text"], json!("☐ 待办"));
    }
}
